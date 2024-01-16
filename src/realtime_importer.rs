use chrono::{Duration, NaiveDateTime, Utc};
use sqlx::{Pool, Postgres, Row};
use tracing::error;

pub struct RealtimeImporter {
    csfloat_last_ts: NaiveDateTime,
    steam_last_ts: NaiveDateTime,
}

impl RealtimeImporter {
    pub fn new() -> RealtimeImporter {
        RealtimeImporter {
            csfloat_last_ts: Utc::now().naive_utc(),
            steam_last_ts: Utc::now().naive_utc() - Duration::hours(24),
        }
    }

    pub async fn get_csfloat_new(&mut self, db: &Pool<Postgres>, size: u32) -> Vec<String> {
        match sqlx::query(
            "SELECT timestamp, response FROM csfloat_responses WHERE timestamp > $1 ORDER BY timestamp LIMIT $2",
        )
        .bind(self.csfloat_last_ts)
        .bind(size as i64)
        .fetch_all(db)
        .await
        {
            Ok(resp) => {
                if let Some(last_row) = resp.last() {
                    self.csfloat_last_ts = last_row.get("timestamp");
                }

                resp.into_iter().map(|x| x.get("response")).collect()
            }
            Err(err) => {
                match err {
                    sqlx::Error::RowNotFound => {},
                    _ => {
                        error!(
                        "Failed to get last csfloat response: {:?}",
                         err
                    );
                }
                }

                vec![]
            },
        }
    }

    pub async fn get_steam_new(&mut self, db: &Pool<Postgres>, size: u32) -> Vec<String> {
        match sqlx::query(
            "SELECT timestamp, response FROM steam_responses WHERE timestamp > $1 ORDER BY timestamp LIMIT $2",
        )
        .bind(self.steam_last_ts)
        .bind(size as i64)
        .fetch_all(db)
        .await
        {
            Ok(resp) => {
                if let Some(last_row) = resp.last() {
                    self.steam_last_ts = last_row.get("timestamp");
                }

                resp.into_iter().map(|x| x.get("response")).collect()
            }
            Err(err) => {
                match err {
                    sqlx::Error::RowNotFound => {},
                    _ => {
                        error!(
                        "Failed to get last steam response: {:?}",
                         err
                    );
                }
                }

                vec![]
            },
        }
    }
}
