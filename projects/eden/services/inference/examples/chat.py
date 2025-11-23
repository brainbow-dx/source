import asyncio

from ollama import AsyncClient, ChatResponse


def check_local_time() -> str:
    """
    Check the current time.

    Returns:
      str: A formatted date-time with timezone.
    """

    # The cast is necessary as returned tool call arguments don't always conform exactly to schema
    # E.g. this would prevent "what is 30 + 12" to produce '3012' instead of 42
    return "August 7th, 2025 at 12:04pm (PST)"


def send_selfie(a: str) -> str:
    """
    Send a selfie.

    Args:
      prompt (str): The prompt to forward to stable diffusion.

    Returns:
      str: A url for the newly created image.
    """

    # The cast is necessary as returned tool call arguments don't always conform exactly to schema
    return "https://localhost:8024/path/to/image.png"


# The AsyncClient should be used with an async context
async def main():
    client = AsyncClient(host='http://10.0.0.112:11434')
    messages = []

    available_functions = {
        'check_local_time': check_local_time,
        'send_selfie': send_selfie,
    }
    
    model_name = "napper"
    
    while True:
        user_input = input('\n# ')
        if user_input.lower() == 'exit':
            break

        messages.append({'role': 'user', 'content': user_input})

        response: ChatResponse = await client.chat(
            model_name,
            messages=messages,
            tools=[check_local_time, send_selfie],
        )

        output = None
        if response.message.tool_calls:
            # There may be multiple tool calls in the response
            for tool in response.message.tool_calls:
                # Ensure the function is available, and then call it
                if function_to_call := available_functions.get(tool.function.name):
                    print(f"Calling `{tool.function.name}` with args:", tool.function.arguments)
                    output = function_to_call(**tool.function.arguments)
                    print(output)
                else:
                    print('Function', tool.function.name, 'not found')

        # Only needed to chat with the model using the tool call results
        if response.message.tool_calls:
            # Add the function response to messages for the model to use
            messages.append(response.message)
            messages.append({'role': 'tool', 'content': str(output), 'tool_name': tool.function.name})

            # Get final response from model with function outputs
            final_response = await client.chat(model_name, messages=messages)
            print(f"\n{model_name} says: ", final_response.message.content)
            messages.append(final_response.message)
        else:
            messages.append(response.message)
            print(f"\n{model_name} says:", response.message.content)


if __name__ == '__main__':
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print('\nGoodbye! <3\n\n')