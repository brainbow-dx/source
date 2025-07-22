# Eden Psst

Portable Say-Something Terminal .. or whatever.

 - An encrypted peer-to-peer messaging tool.
    - Uses the AT protocol (via Atlas) for auth, user data, and federation.
        - Use either public AT servers (bsky, etc) or provide a PDS.
        - $5, $10, $20 directly to us gets a PDS big enough to host a guild.
 - Bot integration over ollama (via Eden) for llm inferrence, image rec, etc.
 - A cli for sending/receiving psst messages.
    - First-time user experience: ```
        $ psst [@(#bot|user.bsky.io)] [.. [-b, -a, ..]] [message]
        # @#bot says: [Some in-persona greeting. Do I know you?]
        ```
 - A chat app for sending/receiving psst messages.
 - A terminal app for working with the psst protocol.