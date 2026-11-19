//! The site's report: its addresses most viewed in the last 30 days, from the counts the site keeps
//! (`website_view`: per address and day, nothing about who viewed).

use async_trait::async_trait;
use ferrum_domain::reports::{ReportSource, col, read_tx};
use ferrum_kernel::{Ctx, Error, Kernel, Result};
use serde_json::{Value, json};
use std::collections::HashMap;

pub struct WebsiteReports;

#[async_trait]
impl ReportSource for WebsiteReports {
    fn catalog(&self) -> Vec<Value> {
        // Every address's count, whoever wrote its page: for those who read every page.
        vec![json!({"name": "site_views", "label": "Site Views", "module": "website", "entity": "WebPage", "params": [], "scoped": false})]
    }

    async fn run(&self, k: &Kernel, ctx: &Ctx, name: &str, _params: &HashMap<String, String>) -> Result<Value> {
        if name != "site_views" {
            return Err(Error::NotFound(format!("report {name}")));
        }
        let mut tx = read_tx(k, ctx).await?;
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "select path, sum(views)::bigint from website_view \
             where tenant_id = $1 and day > (now() at time zone $2)::date - 30 group by path order by 2 desc, 1 limit 50",
        )
        .bind(ctx.tenant)
        .bind(&ctx.schema.timezone)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(json!({
            "columns": [col("path", "Address", "data"), col("views", "Views", "int")],
            "rows": rows.into_iter().map(|(path, views)| json!({"path": path, "views": views})).collect::<Vec<_>>(),
        }))
    }
}
