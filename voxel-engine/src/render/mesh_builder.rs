use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::sync::Arc;
use std::thread::JoinHandle;

use bevy::prelude::Asset;
use bevy::prelude::Material;
use bevy::prelude::Mesh;
use bevy::prelude::Resource;
use cb::channel::Receiver;
use cb::channel::SendError;
use cb::channel::Sender;

use crate::data::registry::RegistryManager;
use crate::data::tile::VoxelId;
use crate::topo::access::ChunkBounds;
use crate::topo::access::ReadAccess;
use crate::topo::chunk::ChunkPos;
use crate::topo::chunk_ref::ChunkRef;

use super::adjacency::AdjacentTransparency;
use super::error::MesherError;
use super::mesh::ChunkMesh;

pub struct MesherOutput {
    pub mesh: Mesh,
}

pub struct Context<'a> {
    pub adjacency: &'a AdjacentTransparency,
    pub registries: &'a RegistryManager,
}

pub trait Mesher: Clone + Send + 'static {
    type Material: Material + Asset;

    fn build<Acc>(
        &self,
        access: &Acc,
        adjacency: Context,
    ) -> Result<MesherOutput, MesherError<Acc::ReadErr>>
    where
        Acc: ReadAccess<ReadType = VoxelId> + ChunkBounds;

    fn material(&self) -> Self::Material;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, dm::From, dm::Into)]
pub struct MeshingTaskId(u32);

pub(crate) struct BuildMeshCommand {
    pub chunk_ref: ChunkRef,
    pub adjacency: Box<AdjacentTransparency>,
    pub id: MeshingTaskId,
}

pub(crate) enum MesherWorkerCommand {
    Build(BuildMeshCommand),
    Shutdown,
}

pub(crate) struct MesherWorkerOutput {
    pos: ChunkPos,
    id: MeshingTaskId,
    output: MesherOutput,
}

pub(crate) struct MesherWorker {
    handle: JoinHandle<()>,
}

impl MesherWorker {
    pub fn spawn<Mat: Material>(
        cmd_receiver: &Receiver<MesherWorkerCommand>,
        mesh_sender: &Sender<MesherWorkerOutput>,
        mesher: &impl Mesher<Material = Mat>,
        registries: Arc<RegistryManager>,
    ) -> Self {
        let cmd_receiver = cmd_receiver.clone();
        let mesh_sender = mesh_sender.clone();
        let mesher = mesher.clone();

        let handle = std::thread::spawn(move || {
            let mut interrupt = false;
            while !interrupt {
                // TODO: error handling
                let cmd = cmd_receiver.recv().unwrap();

                match cmd {
                    MesherWorkerCommand::Shutdown => interrupt = true,
                    MesherWorkerCommand::Build(data) => {
                        // TODO: error handling
                        let mesh = data
                            .chunk_ref
                            .with_read_access(|access| {
                                let cx = Context {
                                    adjacency: &data.adjacency,
                                    registries: registries.as_ref(),
                                };
                                mesher.build(&access, cx).unwrap()
                            })
                            .unwrap();

                        mesh_sender
                            .send(MesherWorkerOutput {
                                pos: data.chunk_ref.pos(),
                                id: data.id,
                                output: mesh,
                            })
                            .unwrap();
                    }
                }
            }
        });

        Self { handle }
    }
}

#[derive(Resource)]
pub struct ParallelMeshBuilder<HQM: Mesher, LQM: Mesher> {
    workers: Vec<MesherWorker>,
    cmd_sender: Sender<MesherWorkerCommand>,
    mesh_receiver: Receiver<MesherWorkerOutput>,
    pending_tasks: hb::HashSet<MeshingTaskId>,
    registries: Arc<RegistryManager>,
    hq_mesher: HQM,
    lq_mesher: LQM,
}

impl<HQM: Mesher, LQM: Mesher> ParallelMeshBuilder<HQM, LQM> {
    fn spawn_workers(
        number: u32,
        cmd_recv: &Receiver<MesherWorkerCommand>,
        mesh_send: &Sender<MesherWorkerOutput>,
        mesher: &HQM,
        registries: Arc<RegistryManager>,
    ) -> Vec<MesherWorker> {
        let mut workers = Vec::new();

        for _ in 0..number {
            let worker = MesherWorker::spawn(cmd_recv, mesh_send, mesher, registries);
            workers.push(worker);
        }

        workers
    }

    pub fn new(hq_mesher: HQM, lq_mesher: LQM, registries: Arc<RegistryManager>) -> Self {
        let num_cpus: usize = std::thread::available_parallelism().unwrap().into();

        // TODO: create these channels in Self::spawn_workers instead
        let (cmd_send, cmd_recv) = cb::channel::unbounded::<MesherWorkerCommand>();
        let (mesh_send, mesh_recv) = cb::channel::unbounded::<MesherWorkerOutput>();

        Self {
            workers: Self::spawn_workers(
                num_cpus as _,
                &cmd_recv,
                &mesh_send,
                &hq_mesher,
                registries.clone(),
            ),
            cmd_sender: cmd_send,
            mesh_receiver: mesh_recv,
            pending_tasks: hb::HashSet::new(),
            registries,
            hq_mesher,
            lq_mesher,
        }
    }

    fn unique_task_id(&self) -> MeshingTaskId {
        let max: u32 = self
            .pending_tasks
            .iter()
            .max()
            .cloned()
            .unwrap_or(0.into())
            .into();
        for id in 0..=(max + 1) {
            if !self.pending_tasks.contains(&MeshingTaskId::from(id)) {
                return id.into();
            }
        }

        panic!("Good luck queuing this many tasks lol");
    }

    fn send_cmd(&self, cmd: MesherWorkerCommand) {
        // TODO: error handling
        self.cmd_sender.send(cmd).unwrap()
    }

    fn add_pending_task(&mut self, id: MeshingTaskId) {
        self.pending_tasks.insert(id);
    }

    fn remove_pending_task(&mut self, id: MeshingTaskId) -> bool {
        self.pending_tasks.remove(&id)
    }

    pub fn queue_chunk(
        &mut self,
        chunk_ref: ChunkRef,
        adjacency: AdjacentTransparency,
    ) -> MeshingTaskId {
        let id = self.unique_task_id();
        self.add_pending_task(id);

        let build_cmd = BuildMeshCommand {
            id,
            chunk_ref,
            adjacency: Box::new(adjacency),
        };

        let cmd = MesherWorkerCommand::Build(build_cmd);
        self.send_cmd(cmd);

        id
    }

    // TODO: make this return an iterator instead
    pub fn finished_meshes(&mut self) -> Vec<ChunkMesh> {
        let mut meshes = Vec::<ChunkMesh>::new();

        while let Ok(worker_response) = self.mesh_receiver.try_recv() {
            self.remove_pending_task(worker_response.id);

            let mesh = ChunkMesh {
                pos: worker_response.pos,
                mesh: worker_response.output.mesh,
            };

            meshes.push(mesh);
        }

        meshes
    }

    pub fn shutdown(self) {
        for _ in 0..self.workers.len() {
            self.send_cmd(MesherWorkerCommand::Shutdown);
        }

        for worker in self.workers.into_iter() {
            worker.handle.join().unwrap();
        }
    }

    pub fn lq_material(&self) -> LQM::Material {
        self.lq_mesher.material()
    }

    pub fn hq_material(&self) -> HQM::Material {
        self.hq_mesher.material()
    }
}