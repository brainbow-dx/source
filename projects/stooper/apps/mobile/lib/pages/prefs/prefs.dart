import "package:flutter/material.dart";

class PrefsScreen extends StatefulWidget {
  const PrefsScreen({super.key});

  @override
  State<PrefsScreen> createState() => _PrefsScreenState();
}

class _PrefsScreenState extends State<PrefsScreen> {
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        // TODO: Go back to the last-known-location.
        // leading: IconButton(
        //   icon: const Icon(Icons.arrow_back),
        //   onPressed: () {
        //     Navigator.of(context).pop();
        //   },
        // ),
        leadingWidth: 0.0,
        automaticallyImplyLeading: false,
        title: const Text('Preferences'),
      ),
      body: const Center(
        child: Column(
          children: [
            Expanded(
              child: Text("TEST"),
            ),
          ],
        ),
      ),
    );
  }
}
