await $`
    docker compose -c compose.yaml
        -c tools/monitor/compose.yaml
        --profile dev --profile monitor
        up -d --build
`;