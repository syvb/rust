//! Manages the dataflow bits required for borrowck.
//!
//! FIXME: this might be better as a "generic" fixed-point combinator,
//! but is not as ugly as it is right now.

#![allow(unused)]

use crate::borrow_check::location::LocationIndex;

use crate::borrow_check::nll::PoloniusOutput;

use crate::dataflow::generic::ResultsCursor;
use crate::dataflow::indexes::BorrowIndex;
use crate::dataflow::move_paths::HasMoveData;
use crate::dataflow::Borrows;
use crate::dataflow::EverInitializedPlaces;
use crate::dataflow::MaybeUninitializedPlaces;
use either::Either;
use std::fmt;
use std::rc::Rc;

crate struct Flows<'b, 'tcx> {
    pub borrows: ResultsCursor<'b, 'tcx, Borrows<'b, 'tcx>>,
    pub uninits: ResultsCursor<'b, 'tcx, MaybeUninitializedPlaces<'b, 'tcx>>,
    pub ever_inits: ResultsCursor<'b, 'tcx, EverInitializedPlaces<'b, 'tcx>>,

    /// Polonius Output
    pub polonius_output: Option<Rc<PoloniusOutput>>,
}

impl<'b, 'tcx> Flows<'b, 'tcx> {
    crate fn new(
        borrows: ResultsCursor<'b, 'tcx, Borrows<'b, 'tcx>>,
        uninits: ResultsCursor<'b, 'tcx, MaybeUninitializedPlaces<'b, 'tcx>>,
        ever_inits: ResultsCursor<'b, 'tcx, EverInitializedPlaces<'b, 'tcx>>,
        polonius_output: Option<Rc<PoloniusOutput>>,
    ) -> Self {
        Flows { borrows, uninits, ever_inits, polonius_output }
    }

    crate fn borrows_in_scope(
        &self,
        location: LocationIndex,
    ) -> impl Iterator<Item = BorrowIndex> + '_ {
        if let Some(ref polonius) = self.polonius_output {
            Either::Left(polonius.errors_at(location).iter().cloned())
        } else {
            Either::Right(self.borrows.get().iter())
        }
    }
}

impl<'b, 'tcx> fmt::Display for Flows<'b, 'tcx> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();

        s.push_str("borrows in effect: [");
        let mut saw_one = false;
        self.borrows.get().iter().for_each(|borrow| {
            if saw_one {
                s.push_str(", ");
            };
            saw_one = true;
            let borrow_data = &self.borrows.analysis().borrows()[borrow];
            s.push_str(&borrow_data.to_string());
        });
        s.push_str("] ");

        /*
        s.push_str("borrows generated: [");
        let mut saw_one = false;
        self.borrows.each_gen_bit(|borrow| {
            if saw_one {
                s.push_str(", ");
            };
            saw_one = true;
            let borrow_data = &self.borrows.analysis().borrows()[borrow];
            s.push_str(&borrow_data.to_string());
        });
        s.push_str("] ");
        */

        s.push_str("uninits: [");
        let mut saw_one = false;
        self.uninits.get().iter().for_each(|mpi_uninit| {
            if saw_one {
                s.push_str(", ");
            };
            saw_one = true;
            let move_path = &self.uninits.analysis().move_data().move_paths[mpi_uninit];
            s.push_str(&move_path.to_string());
        });
        s.push_str("] ");

        s.push_str("ever_init: [");
        let mut saw_one = false;
        self.ever_inits.get().iter().for_each(|mpi_ever_init| {
            if saw_one {
                s.push_str(", ");
            };
            saw_one = true;
            let ever_init = &self.ever_inits.analysis().move_data().inits[mpi_ever_init];
            s.push_str(&format!("{:?}", ever_init));
        });
        s.push_str("]");

        fmt::Display::fmt(&s, fmt)
    }
}
