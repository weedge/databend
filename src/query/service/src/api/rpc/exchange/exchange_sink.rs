// Copyright 2022 Datafuse Labs.
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

use std::sync::Arc;

use common_exception::ErrorCode;
use common_exception::Result;
use common_expression::DataBlock;
use common_pipeline_core::pipe::Pipe;
use common_pipeline_core::pipe::PipeItem;
use common_pipeline_core::processors::processor::ProcessorPtr;

use crate::api::rpc::exchange::exchange_params::ExchangeParams;
use crate::api::rpc::exchange::exchange_sink_writer::create_writer_items;
use crate::api::rpc::exchange::exchange_sink_writer::ExchangeWriterSink;
use crate::api::rpc::exchange::exchange_sorting::ExchangeSorting;
use crate::api::rpc::exchange::exchange_sorting::TransformExchangeSorting;
use crate::api::rpc::exchange::exchange_transform_shuffle::exchange_shuffle;
use crate::api::rpc::exchange::serde::exchange_serializer::create_serializer_items;
use crate::api::rpc::exchange::serde::exchange_serializer::ExchangeSerializeMeta;
use crate::api::rpc::exchange::serde::exchange_serializer::TransformExchangeSerializer;
use crate::api::rpc::exchange::serde::exchange_serializer_with_sorting::TransformExchangeSerializerWithSorting;
use crate::clusters::ClusterHelper;
use crate::pipelines::Pipeline;
use crate::sessions::QueryContext;
use crate::sessions::TableContext;

pub struct ExchangeSink;

impl ExchangeSink {
    pub fn via(
        ctx: &Arc<QueryContext>,
        params: &ExchangeParams,
        pipeline: &mut Pipeline,
    ) -> Result<()> {
        let exchange_manager = ctx.get_exchange_manager();
        let mut flight_exchanges = exchange_manager.get_flight_exchanges(params)?;

        match params {
            ExchangeParams::MergeExchange(params) => {
                if params.destination_id == ctx.get_cluster().local_id() {
                    return Err(ErrorCode::Internal(format!(
                        "Locally depends on merge exchange, but the localhost is not a coordination node. executor: {}, destination_id: {}, fragment id: {}",
                        ctx.get_cluster().local_id(),
                        params.destination_id,
                        params.fragment_id
                    )));
                }

                if let Some(sorting) = &params.exchange_sorting {
                    pipeline.add_transform(|input, output| {
                        Ok(TransformExchangeSerializerWithSorting::create(
                            input,
                            output,
                            &params.schema,
                            sorting.clone(),
                        ))
                    })?;

                    let output_len = pipeline.output_len();
                    let sorting = SinkExchangeSorting::create();

                    let transform = TransformExchangeSorting::create(output_len, sorting);

                    let output = transform.get_output();
                    let inputs = transform.get_inputs();
                    pipeline.add_pipe(Pipe::create(output_len, 1, vec![PipeItem::create(
                        ProcessorPtr::create(Box::new(transform)),
                        inputs,
                        vec![output],
                    )]));
                } else {
                    pipeline.add_transform(|input, output| {
                        Ok(TransformExchangeSerializer::create(
                            input,
                            output,
                            &params.schema,
                        ))
                    })?;
                }

                assert_eq!(flight_exchanges.len(), 1);
                let flight_exchange = flight_exchanges.remove(0);
                pipeline.add_sink(|input| {
                    Ok(ProcessorPtr::create(ExchangeWriterSink::create(
                        input,
                        flight_exchange.clone(),
                    )))
                })
            }
            ExchangeParams::ShuffleExchange(params) => {
                exchange_shuffle(params, pipeline)?;

                // exchange serialize transform
                let len = flight_exchanges.len();
                let items = create_serializer_items(len, &params.schema);
                pipeline.add_pipe(Pipe::create(len, len, items));

                // exchange writer sink
                let items = create_writer_items(flight_exchanges);
                pipeline.add_pipe(Pipe::create(len, 0, items));
                Ok(())
            }
        }
    }
}

struct SinkExchangeSorting {}

impl SinkExchangeSorting {
    pub fn create() -> Arc<dyn ExchangeSorting> {
        Arc::new(SinkExchangeSorting {})
    }
}

impl ExchangeSorting for SinkExchangeSorting {
    fn block_number(&self, data_block: &DataBlock) -> Result<usize> {
        let block_meta = data_block.get_meta();
        let shuffle_meta = block_meta
            .and_then(|meta| meta.as_any().downcast_ref::<ExchangeSerializeMeta>())
            .unwrap();

        Ok(shuffle_meta.block_number)
    }
}
