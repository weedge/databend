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

use common_exception::Result;
use common_meta_app::principal::GrantObject;
use common_meta_app::principal::UserPrivilegeType;

use crate::interpreters::access::AccessChecker;
use crate::sessions::QueryContext;
use crate::sql::plans::Plan;

pub struct PrivilegeAccess {
    ctx: Arc<QueryContext>,
}

impl PrivilegeAccess {
    pub fn create(ctx: Arc<QueryContext>) -> Box<dyn AccessChecker> {
        Box::new(PrivilegeAccess { ctx })
    }
}

#[async_trait::async_trait]
impl AccessChecker for PrivilegeAccess {
    async fn check(&self, plan: &Plan) -> Result<()> {
        let session = self.ctx.get_current_session();

        match plan {
            Plan::Query { metadata, .. } => {
                let metadata = metadata.read().clone();
                for table in metadata.tables() {
                    if table.is_source_of_view() {
                        continue;
                    }
                    session
                        .validate_privilege(
                            &GrantObject::Table(
                                table.catalog().to_string(),
                                table.database().to_string(),
                                table.name().to_string(),
                            ),
                            UserPrivilegeType::Select,
                        )
                        .await?
                }
            }
            Plan::Explain { .. } => {}
            Plan::ExplainAnalyze { .. } => {}
            Plan::Copy(_) => {}
            Plan::Call(_) => {}
            // Catalog
            Plan::ShowCreateCatalog(_) => {}
            Plan::CreateCatalog(_) => {}
            Plan::DropCatalog(_) => {}

            // Database.
            Plan::ShowCreateDatabase(_) => {}
            Plan::CreateDatabase(_) => {
                session
                    .validate_privilege(&GrantObject::Global, UserPrivilegeType::Create)
                    .await?;
            }
            Plan::DropDatabase(_) => {
                session
                    .validate_privilege(&GrantObject::Global, UserPrivilegeType::Drop)
                    .await?;
            }
            Plan::UndropDatabase(_) => {
                session
                    .validate_privilege(&GrantObject::Global, UserPrivilegeType::Drop)
                    .await?;
            }
            Plan::RenameDatabase(_) => {
                session
                    .validate_privilege(&GrantObject::Global, UserPrivilegeType::Alter)
                    .await?;
            }
            Plan::UseDatabase(_) => {}

            // Table.
            Plan::ShowCreateTable(_) => {}
            Plan::DescribeTable(_) => {}
            Plan::CreateTable(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Database(plan.catalog.clone(), plan.database.clone()),
                        UserPrivilegeType::Create,
                    )
                    .await?;
            }
            Plan::DropTable(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Database(plan.catalog.clone(), plan.database.clone()),
                        UserPrivilegeType::Drop,
                    )
                    .await?;
            }
            Plan::UndropTable(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Database(plan.catalog.clone(), plan.database.clone()),
                        UserPrivilegeType::Drop,
                    )
                    .await?;
            }
            Plan::RenameTable(_) => {}
            Plan::AddTableColumn(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Table(
                            plan.catalog.clone(),
                            plan.database.clone(),
                            plan.table.clone(),
                        ),
                        UserPrivilegeType::Alter,
                    )
                    .await?;
            }
            Plan::DropTableColumn(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Table(
                            plan.catalog.clone(),
                            plan.database.clone(),
                            plan.table.clone(),
                        ),
                        UserPrivilegeType::Alter,
                    )
                    .await?;
            }
            Plan::AlterTableClusterKey(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Table(
                            plan.catalog.clone(),
                            plan.database.clone(),
                            plan.table.clone(),
                        ),
                        UserPrivilegeType::Alter,
                    )
                    .await?;
            }
            Plan::DropTableClusterKey(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Table(
                            plan.catalog.clone(),
                            plan.database.clone(),
                            plan.table.clone(),
                        ),
                        UserPrivilegeType::Drop,
                    )
                    .await?;
            }
            Plan::ReclusterTable(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Table(
                            plan.catalog.clone(),
                            plan.database.clone(),
                            plan.table.clone(),
                        ),
                        UserPrivilegeType::Alter,
                    )
                    .await?;
            }
            Plan::TruncateTable(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Table(
                            plan.catalog.clone(),
                            plan.database.clone(),
                            plan.table.clone(),
                        ),
                        UserPrivilegeType::Delete,
                    )
                    .await?;
            }
            Plan::OptimizeTable(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Table(
                            plan.catalog.clone(),
                            plan.database.clone(),
                            plan.table.clone(),
                        ),
                        UserPrivilegeType::Super,
                    )
                    .await?;
            }
            Plan::AnalyzeTable(_) => {}
            Plan::ExistsTable(_) => {}

            // Others.
            Plan::Insert(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Table(
                            plan.catalog.clone(),
                            plan.database.clone(),
                            plan.table.clone(),
                        ),
                        UserPrivilegeType::Insert,
                    )
                    .await?;
            }
            Plan::Delete(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Table(
                            plan.catalog_name.clone(),
                            plan.database_name.clone(),
                            plan.table_name.clone(),
                        ),
                        UserPrivilegeType::Delete,
                    )
                    .await?;
            }
            Plan::Update(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Table(
                            plan.catalog.clone(),
                            plan.database.clone(),
                            plan.table.clone(),
                        ),
                        UserPrivilegeType::Update,
                    )
                    .await?;
            }
            Plan::CreateView(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Database(plan.catalog.clone(), plan.database.clone()),
                        UserPrivilegeType::Alter,
                    )
                    .await?;
            }
            Plan::AlterView(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Database(plan.catalog.clone(), plan.database.clone()),
                        UserPrivilegeType::Alter,
                    )
                    .await?;
            }
            Plan::DropView(plan) => {
                session
                    .validate_privilege(
                        &GrantObject::Database(plan.catalog.clone(), plan.database.clone()),
                        UserPrivilegeType::Drop,
                    )
                    .await?;
            }
            Plan::AlterUser(_) => {}
            Plan::CreateUser(_) => {}
            Plan::DropUser(_) => {}
            Plan::CreateUDF(_) => {}
            Plan::AlterUDF(_) => {}
            Plan::DropUDF(_) => {}
            Plan::CreateRole(_) => {}
            Plan::DropRole(_) => {}
            Plan::GrantRole(_) => {}
            Plan::GrantPriv(_) => {}
            Plan::ShowGrants(_) => {}
            Plan::ShowRoles(_) => {}
            Plan::RevokePriv(_) => {}
            Plan::RevokeRole(_) => {}
            Plan::ListStage(_) => {}
            Plan::CreateStage(_) => {}
            Plan::DropStage(_) => {}
            Plan::RemoveStage(_) => {}
            Plan::CreateFileFormat(_) => {}
            Plan::DropFileFormat(_) => {}
            Plan::ShowFileFormats(_) => {}
            Plan::Presign(_) => {}
            Plan::SetVariable(_) => {}
            Plan::UnSetVariable(_) => {}
            Plan::SetRole(_) => {}
            Plan::Kill(_) => {
                session
                    .validate_privilege(&GrantObject::Global, UserPrivilegeType::Super)
                    .await?;
            }
            Plan::CreateShare(_) => {}
            Plan::DropShare(_) => {}
            Plan::GrantShareObject(_) => {}
            Plan::RevokeShareObject(_) => {}
            Plan::AlterShareTenants(_) => {}
            Plan::DescShare(_) => {}
            Plan::ShowShares(_) => {}
            Plan::ShowObjectGrantPrivileges(_) => {}
            Plan::ShowGrantTenantsOfShare(_) => {}
            Plan::ExplainAst { .. } => {}
            Plan::ExplainSyntax { .. } => {}
            Plan::RevertTable(_) => {
                session
                    .validate_privilege(&GrantObject::Global, UserPrivilegeType::Alter)
                    .await?;
            }
        }

        Ok(())
    }
}
