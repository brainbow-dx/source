let workdir = $env.FILE_PWD | path dirname

let build_script = $workdir | path join '../../scripts/build.tsx';
let docker_compose_json = $workdir | path join '../../../../compose.yaml';
let caddyfile = $workdir | path join '../../../../Caddyfile';

let mcp_entrypoint = $workdir | path join './main.tsx';

deno run -A $build_script

docker compose -f $docker_compose_json up -d

caddy start $caddyfile

deno run -A $mcp_entrypoint
