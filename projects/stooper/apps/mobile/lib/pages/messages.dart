import "package:flutter/foundation.dart";
import "package:flutter/material.dart";

class InboxScreen extends StatelessWidget {
  const InboxScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Inbox'),
        automaticallyImplyLeading: false,
        leading: IconButton(
          icon: const Icon(Icons.filter_list),
          onPressed: () {
            if (kDebugMode) {
              print('Dang!');
            }
          },
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.settings),
            onPressed: () {
              //..
            },
          ),
        ],
      ),
      body: const Center(
        child: Column(
          children: [
            Text("Notifications + Messages"),
          ],
        ),
      ),
    );
  }
}
