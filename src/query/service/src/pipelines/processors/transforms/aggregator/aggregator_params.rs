// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::alloc::Layout;
use std::sync::Arc;

use common_exception::Result;
use common_expression::types::DataType;
use common_expression::DataBlock;
use common_expression::DataSchemaRef;
use common_expression::HashMethodKind;
use common_functions::aggregates::get_layout_offsets;
use common_functions::aggregates::AggregateFunctionRef;
use common_functions::aggregates::StateAddr;
use common_sql::IndexType;

use crate::pipelines::processors::port::InputPort;
use crate::pipelines::processors::port::OutputPort;
use crate::pipelines::processors::transforms::group_by::Area;

pub struct AggregatorParams {
    pub input_schema: DataSchemaRef,
    pub group_columns: Vec<IndexType>,
    pub group_data_types: Vec<DataType>,

    pub aggregate_functions: Vec<AggregateFunctionRef>,
    pub aggregate_functions_arguments: Vec<Vec<usize>>,

    // about function state memory layout
    // If there is no aggregate function, layout is None
    pub layout: Option<Layout>,
    pub offsets_aggregate_states: Vec<usize>,

    // Limit is push down to AggregatorTransform
    pub limit: Option<usize>,
}

impl AggregatorParams {
    pub fn try_create(
        input_schema: DataSchemaRef,
        group_data_types: Vec<DataType>,
        group_columns: &[usize],
        agg_funcs: &[AggregateFunctionRef],
        agg_args: &[Vec<usize>],
        limit: Option<usize>,
    ) -> Result<Arc<AggregatorParams>> {
        let mut states_offsets: Vec<usize> = Vec::with_capacity(agg_funcs.len());
        let mut states_layout = None;
        if !agg_funcs.is_empty() {
            states_offsets = Vec::with_capacity(agg_funcs.len());
            states_layout = Some(get_layout_offsets(agg_funcs, &mut states_offsets)?);
        }

        Ok(Arc::new(AggregatorParams {
            input_schema,
            group_columns: group_columns.to_vec(),
            group_data_types,
            aggregate_functions: agg_funcs.to_vec(),
            aggregate_functions_arguments: agg_args.to_vec(),
            layout: states_layout,
            offsets_aggregate_states: states_offsets,
            limit,
        }))
    }

    pub fn alloc_layout(&self, area: &mut Area) -> StateAddr {
        let layout = self.layout.unwrap();
        let place = Into::<StateAddr>::into(area.alloc_layout(layout));

        for idx in 0..self.offsets_aggregate_states.len() {
            let aggr_state = self.offsets_aggregate_states[idx];
            let aggr_state_place = place.next(aggr_state);
            self.aggregate_functions[idx].init_state(aggr_state_place);
        }
        place
    }

    pub fn has_distinct_combinator(&self) -> bool {
        self.aggregate_functions
            .iter()
            .any(|f| f.name().contains("DistinctCombinator"))
    }
}

pub struct AggregatorTransformParams {
    pub method: HashMethodKind,
    pub transform_input_port: Arc<InputPort>,
    pub transform_output_port: Arc<OutputPort>,
    pub aggregator_params: Arc<AggregatorParams>,
}

impl AggregatorTransformParams {
    pub fn try_create(
        transform_input_port: Arc<InputPort>,
        transform_output_port: Arc<OutputPort>,
        aggregator_params: &Arc<AggregatorParams>,
    ) -> Result<AggregatorTransformParams> {
        let group_cols = &aggregator_params.group_columns;
        let schema_before_group_by = aggregator_params.input_schema.clone();
        let sample_block = DataBlock::empty_with_schema(schema_before_group_by);
        let method = DataBlock::choose_hash_method(&sample_block, group_cols)?;

        Ok(AggregatorTransformParams {
            method,
            transform_input_port,
            transform_output_port,
            aggregator_params: aggregator_params.clone(),
        })
    }
}
