// Copyright 2023 Datafuse Labs.
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

use std::io::Cursor;
use std::sync::Arc;

use common_arrow::arrow::io::parquet::read::infer_schema;
use common_arrow::arrow::io::parquet::read::{self as pread};
use common_arrow::parquet::read::read_metadata;
use common_catalog::table_context::TableContext;
use common_exception::Result;
use common_expression::DataBlock;
use common_expression::DataSchema;
use common_expression::TableSchema;
use common_meta_store::MetaStore;
use common_storage::DataOperator;
use opendal::Operator;

use crate::common::gen_result_cache_meta_key;
use crate::meta_manager::ResultCacheMetaManager;

pub struct ResultCacheReader {
    meta_mgr: ResultCacheMetaManager,

    operator: Operator,
    /// To ensure the cache is valid.
    partitions_shas: Vec<String>,

    /// If true, the cache will be used even if it is inconsistent.
    /// In another word, `partitions_sha` will not be checked.
    tolerate_inconsistent: bool,
}

impl ResultCacheReader {
    pub fn create(
        ctx: Arc<dyn TableContext>,
        key: &str,
        kv_store: Arc<MetaStore>,
        tolerate_inconsistent: bool,
    ) -> Self {
        let tenant = ctx.get_tenant();
        let meta_key = gen_result_cache_meta_key(&tenant, key);
        let partitions_shas = ctx.get_partitions_shas();

        Self {
            meta_mgr: ResultCacheMetaManager::create(kv_store, meta_key, 0),
            partitions_shas,
            operator: DataOperator::instance().operator(),
            tolerate_inconsistent,
        }
    }

    pub async fn try_read_cached_result(&self) -> Result<Option<Vec<DataBlock>>> {
        match self.meta_mgr.get().await? {
            Some(value) => {
                if self.tolerate_inconsistent || value.partitions_shas == self.partitions_shas {
                    if value.num_rows == 0 {
                        Ok(Some(vec![DataBlock::empty()]))
                    } else {
                        Ok(Some(self.read_result_from_cache(&value.location).await?))
                    }
                } else {
                    // The cache is invalid (due to data update or other reasons).
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    async fn read_result_from_cache(&self, location: &str) -> Result<Vec<DataBlock>> {
        let object = self.operator.object(location);
        let data = object.read().await?;
        let mut reader = Cursor::new(data);
        let meta = read_metadata(&mut reader)?;
        let arrow_schema = infer_schema(&meta)?;
        let schema = DataSchema::from(&TableSchema::from(&arrow_schema));

        // Read the parquet file into one block.
        let chunks_iter =
            pread::FileReader::new(reader, meta.row_groups, arrow_schema, None, None, None);
        let mut blocks = Vec::with_capacity(1);

        for chunk in chunks_iter {
            let block = DataBlock::from_arrow_chunk(&chunk?, &schema)?;
            blocks.push(block);
        }

        Ok(blocks)
    }
}
