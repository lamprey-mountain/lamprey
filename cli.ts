import yargs from "https://deno.land/x/yargs@v17.7.2-deno/deno.ts";
import { Arguments } from "https://deno.land/x/yargs@v17.7.2-deno/deno-types.ts";

console.log(yargs(Deno.args)
  .command('download <files...>', 'download a list of files', (yargs: any) => {
    return yargs.positional('files', {
      describe: 'a list of files to do something with'
    })
  }, (argv: Arguments) => {
    console.info(argv)
  })
  .strictCommands()
  .demandCommand(1)
  .argv)
