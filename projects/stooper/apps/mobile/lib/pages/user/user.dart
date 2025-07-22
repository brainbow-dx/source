import "package:flutter/material.dart";

import "package:provider/provider.dart";

import "package:stooper_mobile/providers/user.dart";

class UserScreen extends StatefulWidget {
  const UserScreen({super.key});

  @override
  State<UserScreen> createState() => _UserScreenState();
}

class _UserScreenState extends State<UserScreen> {
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
        Text('Current User: ${currentUser.name}'),
        const SizedBox(height: 16),
        ElevatedButton(
          onPressed: () {
            Navigator.of(context).pushNamed('/user/settings');
          },
          child: const Text('Settings'),
        ),
      ],
    );
  }
}
