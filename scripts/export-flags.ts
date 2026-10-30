// Exporta el catálogo del referente sin modificar su repositorio.
import { resolve } from 'node:path';
const source = process.argv[2] ?? '/home/projects/plane-llama-bench/front/src/app/core/data/llama-flags.ts';
const { LLAMA_FLAGS } = await import(resolve(source));
await Bun.write(new URL('../assets/llama-flags.json', import.meta.url), JSON.stringify(LLAMA_FLAGS, null, 2) + '\n');
console.log(`${LLAMA_FLAGS.length} flags exportadas.`);
