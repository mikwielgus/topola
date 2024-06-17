use petgraph::graph::EdgeIndex;

use crate::{
    autorouter::{Autorouter, AutorouterError, AutorouterStatus},
    board::mesadata::MesadataTrait,
    router::navmesh::Navmesh,
};

pub struct Autoroute {
    ratlines_iter: Box<dyn Iterator<Item = EdgeIndex<usize>>>,
    navmesh: Option<Navmesh>, // Useful for debugging.
    cur_ratline: Option<EdgeIndex<usize>>,
}

impl Autoroute {
    pub fn new(
        ratlines: impl IntoIterator<Item = EdgeIndex<usize>> + 'static,
        autorouter: &mut Autorouter<impl MesadataTrait>,
    ) -> Result<Self, AutorouterError> {
        let mut ratlines_iter = Box::new(ratlines.into_iter());

        let Some(cur_ratline) = ratlines_iter.next() else {
            return Err(AutorouterError::NothingToRoute);
        };

        let (source, target) = autorouter.ratline_endpoints(cur_ratline);
        let navmesh = Some(Navmesh::new(autorouter.board.layout(), source, target)?);

        let this = Self {
            ratlines_iter,
            navmesh,
            cur_ratline: Some(cur_ratline),
        };

        Ok(this)
    }

    pub fn step<M: MesadataTrait>(
        &mut self,
        autorouter: &mut Autorouter<M>,
    ) -> Result<AutorouterStatus, AutorouterError> {
        let (new_navmesh, new_ratline) = if let Some(cur_ratline) = self.ratlines_iter.next() {
            let (source, target) = autorouter.ratline_endpoints(cur_ratline);

            (
                Some(
                    Navmesh::new(autorouter.board.layout(), source, target)
                        .ok()
                        .unwrap(),
                ),
                Some(cur_ratline),
            )
        } else {
            (None, None)
        };

        match autorouter.board.route_band(
            std::mem::replace(&mut self.navmesh, new_navmesh).unwrap(),
            100.0,
        ) {
            Ok(band) => {
                autorouter
                    .ratsnest
                    .assign_band_to_ratline(self.cur_ratline.unwrap(), band);
                self.cur_ratline = new_ratline;

                if self.navmesh.is_some() {
                    Ok(AutorouterStatus::Running)
                } else {
                    Ok(AutorouterStatus::Finished)
                }
            }
            Err(err) => Err(AutorouterError::Router(err)),
        }
    }

    pub fn navmesh(&self) -> &Option<Navmesh> {
        &self.navmesh
    }
}
