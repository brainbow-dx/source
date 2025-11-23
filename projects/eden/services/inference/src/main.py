import sys

import asyncio

import eden_inference_service
from eden_inference_service import InferenceService;


async def main(args: map):
    print('Hello from Python! <3')
    eden_inference_service.init_runtime(1, 3424)
    
    service_instance = InferenceService()
    service_instance.get_some_string_thing(39, "lorren", None)


if __name__ == '__main__':
    try:
        asyncio.run(main({ }))
    except KeyboardInterrupt:
        print('\nGoodbye! <3\n\n')