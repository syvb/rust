use rustc::mir::{self, BasicBlock, Location};
use rustc_index::bit_set::BitSet;

use super::{Analysis, Results};
use crate::dataflow::impls::{borrows::Borrows, EverInitializedPlaces, MaybeUninitializedPlaces};

/// For every location in a `mir::Body`, calls the corresponding method in `ResultsVisitor` with
/// the appropriate dataflow state in `Results`.
pub fn visit_results<F>(
    body: &'mir mir::Body<'tcx>,
    blocks: impl IntoIterator<Item = BasicBlock>,
    results: &impl ResultsVisitable<'tcx, FlowState = F>,
    vis: &mut impl ResultsVisitor<'mir, 'tcx, FlowState = F>,
) {
    let mut state = results.new_flow_state(body);

    for block in blocks {
        let block_data = &body[block];
        results.reset_to_block_start(&mut state, block);

        for (statement_index, stmt) in block_data.statements.iter().enumerate() {
            let loc = Location { block, statement_index };

            results.reconstruct_before_statement_effect(&mut state, stmt, loc);
            vis.visit_statement(&mut state, stmt, loc);

            results.reconstruct_statement_effect(&mut state, stmt, loc);
            vis.visit_statement_exit(&mut state, stmt, loc);
        }

        let loc = body.terminator_loc(block);
        let term = block_data.terminator();

        results.reconstruct_before_terminator_effect(&mut state, term, loc);
        vis.visit_terminator(&mut state, term, loc);

        results.reconstruct_terminator_effect(&mut state, term, loc);
        vis.visit_terminator_exit(&mut state, term, loc);
    }
}

pub trait ResultsVisitor<'mir, 'tcx> {
    type FlowState;

    fn visit_statement(
        &mut self,
        _state: &Self::FlowState,
        _statement: &'mir mir::Statement<'tcx>,
        _location: Location,
    ) {
    }

    fn visit_statement_exit(
        &mut self,
        _state: &Self::FlowState,
        _statement: &'mir mir::Statement<'tcx>,
        _location: Location,
    ) {
    }

    fn visit_terminator(
        &mut self,
        _state: &Self::FlowState,
        _terminator: &'mir mir::Terminator<'tcx>,
        _location: Location,
    ) {
    }

    fn visit_terminator_exit(
        &mut self,
        _state: &Self::FlowState,
        _terminator: &'mir mir::Terminator<'tcx>,
        _location: Location,
    ) {
    }
}

/// Things that can be visited by a `ResultsVisitor`.
///
/// This trait exists so that we can visit the results of multiple dataflow analyses simultaneously.
pub trait ResultsVisitable<'tcx> {
    type FlowState;

    fn new_flow_state(&self, body: &mir::Body<'tcx>) -> Self::FlowState;

    fn reset_to_block_start(&self, state: &mut Self::FlowState, block: BasicBlock);

    fn reconstruct_before_statement_effect(
        &self,
        state: &mut Self::FlowState,
        statement: &mir::Statement<'tcx>,
        location: Location,
    );

    fn reconstruct_statement_effect(
        &self,
        state: &mut Self::FlowState,
        statement: &mir::Statement<'tcx>,
        location: Location,
    );

    fn reconstruct_before_terminator_effect(
        &self,
        state: &mut Self::FlowState,
        terminator: &mir::Terminator<'tcx>,
        location: Location,
    );

    fn reconstruct_terminator_effect(
        &self,
        state: &mut Self::FlowState,
        terminator: &mir::Terminator<'tcx>,
        location: Location,
    );
}

impl<'tcx, A> ResultsVisitable<'tcx> for Results<'tcx, A>
where
    A: Analysis<'tcx>,
{
    type FlowState = BitSet<A::Idx>;

    fn new_flow_state(&self, body: &mir::Body<'tcx>) -> Self::FlowState {
        BitSet::new_empty(self.analysis.bits_per_block(body))
    }

    fn reset_to_block_start(&self, state: &mut Self::FlowState, block: BasicBlock) {
        state.overwrite(&self.entry_set_for_block(block));
    }

    fn reconstruct_before_statement_effect(
        &self,
        state: &mut Self::FlowState,
        stmt: &mir::Statement<'tcx>,
        loc: Location,
    ) {
        self.analysis.apply_before_statement_effect(state, stmt, loc);
    }

    fn reconstruct_statement_effect(
        &self,
        state: &mut Self::FlowState,
        stmt: &mir::Statement<'tcx>,
        loc: Location,
    ) {
        self.analysis.apply_statement_effect(state, stmt, loc);
    }

    fn reconstruct_before_terminator_effect(
        &self,
        state: &mut Self::FlowState,
        term: &mir::Terminator<'tcx>,
        loc: Location,
    ) {
        self.analysis.apply_before_terminator_effect(state, term, loc);
    }

    fn reconstruct_terminator_effect(
        &self,
        state: &mut Self::FlowState,
        term: &mir::Terminator<'tcx>,
        loc: Location,
    ) {
        self.analysis.apply_terminator_effect(state, term, loc);
    }
}

macro_rules! results_tuples {
    ( $(
        $( #[$meta:meta] )*
        $T:ident { $( $field:ident : $A:ident ),* $(,)? }
    )* ) => { $(
        $( #[$meta] )*
        #[derive(Debug)]
        pub struct $T<$($A),*> {
            $( pub $field: $A, )*
        }

        impl<'tcx, $( $A),*> ResultsVisitable<'tcx> for $T<$( Results<'tcx, $A> ),*>
        where
            $( $A: Analysis<'tcx>, )*
        {
            type FlowState = $T<$( BitSet<$A::Idx> ),*>;

            fn new_flow_state(&self, body: &mir::Body<'tcx>) -> Self::FlowState {
                $T {
                    $( $field: BitSet::new_empty(self.$field.analysis.bits_per_block(body)) ),*
                }
            }

            fn reset_to_block_start(
                &self,
                state: &mut Self::FlowState,
                block: BasicBlock,
            ) {
                $( state.$field.overwrite(&self.$field.entry_sets[block]); )*
            }

            fn reconstruct_before_statement_effect(
                &self,
                state: &mut Self::FlowState,
                stmt: &mir::Statement<'tcx>,
                loc: Location,
            ) {
                $( self.$field.analysis
                    .apply_before_statement_effect(&mut state.$field, stmt, loc); )*
            }

            fn reconstruct_statement_effect(
                &self,
                state: &mut Self::FlowState,
                stmt: &mir::Statement<'tcx>,
                loc: Location,
            ) {
                $( self.$field.analysis
                    .apply_statement_effect(&mut state.$field, stmt, loc); )*
            }

            fn reconstruct_before_terminator_effect(
                &self,
                state: &mut Self::FlowState,
                term: &mir::Terminator<'tcx>,
                loc: Location,
            ) {
                $( self.$field.analysis
                    .apply_before_terminator_effect(&mut state.$field, term, loc); )*
            }

            fn reconstruct_terminator_effect(
                &self,
                state: &mut Self::FlowState,
                term: &mir::Terminator<'tcx>,
                loc: Location,
            ) {
                $( self.$field.analysis
                    .apply_terminator_effect(&mut state.$field, term, loc); )*
            }
        }
    )* }
}

results_tuples!(
    /// A tuple with named fields to hold the results of the dataflow analyses used by the
    /// borrow checker.
    BorrowckAnalyses { borrows: B, uninits: U, ever_inits: E }
);

pub type BorrowckResults<'mir, 'tcx> = BorrowckAnalyses<
    Results<'tcx, Borrows<'mir, 'tcx>>,
    Results<'tcx, MaybeUninitializedPlaces<'mir, 'tcx>>,
    Results<'tcx, EverInitializedPlaces<'mir, 'tcx>>,
>;
