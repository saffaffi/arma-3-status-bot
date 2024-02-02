//! arma-3-status-bot

#![warn(missing_docs, rust_2018_idioms)]
#![cfg_attr(
    doc,
    warn(
        rustdoc::bare_urls,
        rustdoc::broken_intra_doc_links,
        rustdoc::invalid_codeblock_attributes,
        rustdoc::invalid_rust_codeblocks,
        rustdoc::missing_crate_level_docs
    )
)]

use arma_3_status_bot::run_bot;

/// Run arma-3-status-bot.
#[tokio::main]
async fn main() {
    run_bot().await;
}
