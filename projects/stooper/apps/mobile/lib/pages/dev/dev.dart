// import 'dart:math';

import "package:flutter/foundation.dart";
import "package:flutter/material.dart";

import "package:confetti/confetti.dart";

class DevScreen extends StatefulWidget {
  const DevScreen({super.key});

  @override
  State<DevScreen> createState() => _DevScreenState();
}

class _DevScreenState extends State<DevScreen> {
  late ConfettiController _confettiController;
  @override
  void initState() {
    super.initState();
    _confettiController =
        ConfettiController(duration: const Duration(seconds: 10));
  }

  @override
  void dispose() {
    _confettiController.dispose();
    super.dispose();
  }

  // Function to show the confirmation dialog
  Future<bool?> _showConfirmationDialog(BuildContext context) async {
    return showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (BuildContext context) {
        return AlertDialog(
          title: const Text('Reset App'),
          content: const Text('Are you sure??'),
          actions: <Widget>[
            TextButton(
              onPressed: () {
                Navigator.of(context).pop(false);
              },
              child: const Text('No'),
            ),
            TextButton(
              onPressed: () {
                if (kDebugMode) {
                  print('TODO: Clear user data ..');
                }
              },
              child: const Text('Yes'),
            ),
          ],
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            ConfettiWidget(
              confettiController: _confettiController,
              blastDirectionality: BlastDirectionality.explosive,
              // blastDirection: pi / -2,
              // numberOfParticles: 10,
              // displayTarget: false,
              minimumSize: const Size(10, 5),
              maximumSize: const Size(20, 10),
              gravity: 0.005,
              shouldLoop: false,
            ),
            const Text('All Systems Go.'),
            ElevatedButton(
              onPressed: () {
                // Confirm?
                _showConfirmationDialog(context);
              },
              child: const Text('Reset'),
            ),
            ElevatedButton(
              onPressed: () {
                _confettiController.play();
              },
              child: const Text('Party!'),
            ),
          ],
        ),
      ),
    );
  }
}
