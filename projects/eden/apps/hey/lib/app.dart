import 'dart:io';

import 'package:eden_hey/screens/loading.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:logging/logging.dart';

import 'package:window_manager/window_manager.dart';

import 'package:provider/provider.dart';

Future<Widget> setupRuntime([Widget? app]) async {
  Logger.root.level = Level.ALL;
  Logger.root.onRecord.listen((record) {
    if (kDebugMode) {
      print('${record.level.name}: ${record.time}: ${record.message}');
    }
  });

  WidgetsFlutterBinding.ensureInitialized();

  if (!kIsWeb) {
    if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
      await windowManager.ensureInitialized();
      await windowManager.setAlwaysOnTop(true);
      // await windowManager.setSize(Size(800.0, 600.0));
      // await windowManager.setBadgeLabel("test");
    }
  }

  return MultiProvider(
    providers: [
      // ChangeNotifierProvider(
      //   create: (context) => UserProvider(store: UserStore()),
      // ),
      // ChangeNotifierProvider(create: (context) => LocationProvider()),
    ],
    child: app ?? const Application(),
  );
}

class Application extends StatelessWidget {
  const Application({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Hey',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        brightness: Brightness.light,
        primarySwatch: Colors.blue,
      ),
      darkTheme: ThemeData(
        brightness: Brightness.dark,
        primarySwatch: Colors.blue,
      ),
      themeMode: ThemeMode.dark,
      home: const LoadingScreen(title: 'Hey'),
    );
  }
}
