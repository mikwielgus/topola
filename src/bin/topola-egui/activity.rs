use topola::{
    autorouter::{
        execute::Execute,
        invoker::{
            GetGhosts, GetMaybeNavmesh, GetMaybeTrace, GetObstacles, Invoker, InvokerError,
            InvokerStatus,
        },
    },
    board::mesadata::AccessMesadata,
    drawing::graph::PrimitiveIndex,
    geometry::primitive::PrimitiveShape,
    router::{navmesh::Navmesh, trace::Trace},
    step::Step,
};

pub struct ActivityWithStatus {
    execute: Execute,
    maybe_status: Option<InvokerStatus>,
}

impl ActivityWithStatus {
    pub fn new_execute(execute: Execute) -> ActivityWithStatus {
        Self {
            execute,
            maybe_status: None,
        }
    }

    pub fn step<M: AccessMesadata>(
        &mut self,
        invoker: &mut Invoker<M>,
    ) -> Result<InvokerStatus, InvokerError> {
        let status = self.execute.step(invoker)?;
        self.maybe_status = Some(status.clone());
        Ok(status)
    }

    pub fn maybe_status(&self) -> Option<InvokerStatus> {
        self.maybe_status.clone()
    }
}

impl GetMaybeNavmesh for ActivityWithStatus {
    fn maybe_navmesh(&self) -> Option<&Navmesh> {
        self.execute.maybe_navmesh()
    }
}

impl GetMaybeTrace for ActivityWithStatus {
    fn maybe_trace(&self) -> Option<&Trace> {
        self.execute.maybe_trace()
    }
}

impl GetGhosts for ActivityWithStatus {
    fn ghosts(&self) -> &[PrimitiveShape] {
        self.execute.ghosts()
    }
}

impl GetObstacles for ActivityWithStatus {
    fn obstacles(&self) -> &[PrimitiveIndex] {
        self.execute.obstacles()
    }
}
