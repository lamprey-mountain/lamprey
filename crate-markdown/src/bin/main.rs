use clap::{Parser, Subcommand};

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Tokenize { source } => {
            let mut lexer = lamprey_markdown::lexer::Lexer::new(&source);
            while let Some(next) = lexer.advance() {
                dbg!(next);
            }
        }
        Command::Parse { source } => {
            let parser = lamprey_markdown::Parser::new();
            let parsed = parser.parse(&source);
            dbg!(parsed.ast());
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
    /// tokenize some markdown (lexer)
    Tokenize { source: String },

    /// parse some markdown
    Parse {
        // #[arg(short, long)]
        // target: Target,
        source: String,
    },
}

// #[derive(Debug)]
// enum Target {
//     Ast,
// }
