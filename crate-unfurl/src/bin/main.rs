use clap::{Parser, Subcommand};
use lamprey_unfurl::{
    DirectMediaPlugin, HtmlStreamPlugin, Unfurler,
    plugin::wikipedia::WikipediaPlugin,
    unfurler::{EmbedGeneration, PrettyEmbedGeneration},
};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Unfurl { url } => {
            let unfurler = Unfurler::builder()
                .add_plugin(WikipediaPlugin::default())
                .add_plugin(HtmlStreamPlugin {
                    max_bytes: 1024 * 1024 * 4,
                })
                .add_plugin(DirectMediaPlugin)
                .build()
                .expect("failed to build unfurler");
            match unfurler.unfurl(&url).await {
                Ok(embeds) => {
                    if embeds.is_empty() {
                        println!("no embeds found");
                    } else {
                        println!("found {} embeds", embeds.len());
                        for e in embeds {
                            println!("{:#?}", PrettyEmbedGeneration(&e));
                        }
                    }
                }
                Err(err) => println!("error while unfurling: {err:?}"),
            }
        }
    }

    Ok(())
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// unfurl a url
    Unfurl { url: Url },
}
