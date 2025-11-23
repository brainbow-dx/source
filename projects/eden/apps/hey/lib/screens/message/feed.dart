import 'dart:math';

import 'package:flutter/material.dart';

import 'package:flutter_chat_core/flutter_chat_core.dart';
import 'package:flutter_chat_ui/flutter_chat_ui.dart';

import 'package:ollama/ollama.dart';

class MessageFeed extends StatefulWidget {
  const MessageFeed({super.key});

  @override
  MessageFeedState createState() => MessageFeedState();
}

class MessageFeedState extends State<MessageFeed> {
  final _chatController = InMemoryChatController();
  final _ollamaController = Ollama();

  @override
  void dispose() {
    _chatController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Chat(
        chatController: _chatController,
        currentUserId: '@moodring.dev',
        theme: ChatTheme.fromThemeData(Theme.of(context)),
        builders: Builders(),
        resolveUser: (UserID id) async {
          return User(id: id, name: 'Lorren');
        },
        onMessageSend: (text) async {
          _chatController.insertMessage(
            TextMessage(
              id: '${Random().nextInt(1000) + 1}',
              authorId: '@moodring.dev',
              createdAt: DateTime.now().toUtc(),
              text: text,
            ),
          );

          final stream = _ollamaController.generate(text, model: 'sarah');

          final messageBuffer = StringBuffer();

          await for (final chunk in stream) {
            String message = chunk.toString();
            if (message.isNotEmpty) {
              messageBuffer.write(message);
            }
          }

          if (messageBuffer.length > 0) {
            _chatController.insertMessage(
              TextMessage(
                // Better to use UUID or similar for the ID - IDs must be unique
                id: '${Random().nextInt(1000) + 1}',
                authorId: '@happy_napper345',
                createdAt: DateTime.now().toUtc(),
                text: messageBuffer.toString(),
              ),
            );
          }
        },
      ),
    );
  }
}
