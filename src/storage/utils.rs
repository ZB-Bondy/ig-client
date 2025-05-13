
use crate::application::models::transaction::Transaction;
use crate::error::AppError;
use sqlx::Executor;  

pub async fn store_transactions(
    pool: &sqlx::PgPool,
    txs: &[Transaction],
) -> Result<usize, AppError> {
    let mut tx = pool.begin().await?;
    let mut inserted = 0;

    for t in txs {
        let result = tx
            .execute(
                sqlx::query(
                    r#"
                    INSERT INTO ig_options (
                        reference, deal_date, underlying, strike,
                        option_type, expiry, transaction_type, pnl_eur, is_fee, raw
                    )
                    VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                    ON CONFLICT (raw_hash) DO NOTHING
                    "#
                )
                    .bind(&t.reference)
                    .bind(t.deal_date)
                    .bind(&t.underlying)
                    .bind(t.strike)
                    .bind(&t.option_type)
                    .bind(t.expiry)
                    .bind(&t.transaction_type)
                    .bind(t.pnl_eur)
                    .bind(t.is_fee)
                    .bind(&t.raw_json),
            )
            .await?;

        inserted += result.rows_affected() as usize;
    }

    tx.commit().await?;
    Ok(inserted)
}