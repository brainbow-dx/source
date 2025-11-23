import "package:flutter/material.dart";

import "package:provider/provider.dart";

import "package:stooper_mobile/providers/user.dart";

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  @override
  Widget build(BuildContext context) {
    final userProvider = context.watch<UserProvider>();
    final currentUser = userProvider.getUser('TODO');

    if (currentUser == null) {
      return const Center(child: Text('No Current User'));
    }

    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        const Text('Home'),
        const SizedBox(height: 16),
        Text('Current User: @${currentUser.name}'),
        const SizedBox(height: 16),
        ElevatedButton(
          onPressed: () {
            Navigator.of(context).pushNamed('/prefs');
          },
          child: const Text('Settings'),
        ),
      ],
    );
  }
}
