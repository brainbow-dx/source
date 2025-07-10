import "package:flutter/material.dart";

import "package:provider/provider.dart";

import "package:stooper_mobile/providers/user.dart";

class UserScreen extends StatelessWidget {
  const UserScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final userProvider = context.watch<UserProvider>();
    final currentUser = userProvider.getUser('TODO');

    return (currentUser == null)
        ? const Text('No Current User')
        : Text('Current User: ${currentUser.name}');
  }
}
