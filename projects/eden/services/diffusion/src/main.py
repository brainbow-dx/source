import sys

import asyncio

import service


async def main(args: map):
    await service.some_function()


if __name__ == '__main__':
    try:
        # TODO: Parse args ..
        asyncio.run(main({ }))
    
    except KeyboardInterrupt:
        print('\nGoodbye! <3\n\n')