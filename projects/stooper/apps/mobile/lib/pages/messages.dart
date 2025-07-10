import "package:flutter/foundation.dart";
import "package:flutter/material.dart";

class InboxScreen extends StatelessWidget {
  const InboxScreen({super.key});

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
      body: Center(
        child: Column(
          children: const [
            Text("Notifications + Messages"),
          ],
        ),
      ),
    );
  }
}
