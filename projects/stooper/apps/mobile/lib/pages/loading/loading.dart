import "package:flutter/foundation.dart";
import "package:flutter/material.dart";

import "package:provider/provider.dart";

import "package:stooper_mobile/providers/user.dart";

class LoadingScreen extends StatefulWidget {
  const LoadingScreen({super.key});

  @override
  State<LoadingScreen> createState() => _LoadingScreenState();
}

class _LoadingScreenState extends State<LoadingScreen> {
  bool? _isLoading = true;

  @override
  void initState() {
    super.initState();
    _fetchData();
  }

  Future<void> _fetchData() async {
    await Future.delayed(const Duration(seconds: 1));
    setState(() {
      _isLoading = false;
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (kDebugMode) {
        print('Is Loading? $_isLoading');
      }
      Navigator.of(context).pushNamed("/feed");
    });
  }

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
        const Text('Loading ..'),
        const SizedBox(height: 16),
        ElevatedButton(
          onPressed: () {
            Navigator.of(context).pushNamed('/feed');
          },
          child: const Text('Go to feed ..'),
        ),
        ElevatedButton(
          onPressed: () {
            Navigator.of(context).pushNamed('/home');
          },
          child: const Text('Go home ..'),
        ),
      ],
    );
  }
}
