import "package:flutter/foundation.dart";
import "package:flutter/material.dart";
import "package:stooper_mobile/pages/feed/list.dart";

class FeedScreen extends StatefulWidget {
  const FeedScreen({super.key});

  // This widget is the home page of your application. It is stateful, meaning
  // that it has a State object (defined below) that contains fields that affect
  // how it looks.

  // This class is the configuration for the state. It holds the values (in this
  // case the title) provided by the parent (in this case the App widget) and
  // used by the build method of the State. Fields in a Widget subclass are
  // always marked "final".

  @override
  State<FeedScreen> createState() => _FeedScreenState();
}

class _FeedScreenState extends State<FeedScreen> {
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: Column(
          children: const [
            Expanded(
              child: FeedList(),
            ),
          ],
        ),
      ),
    );
  }
}
