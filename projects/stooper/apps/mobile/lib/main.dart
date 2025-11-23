import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:phosphor_flutter/phosphor_flutter.dart';

import 'package:window_manager/window_manager.dart';

import 'package:logging/logging.dart';

import 'package:provider/provider.dart';

import 'package:stooper_mobile/providers/location.dart';
import 'package:stooper_mobile/pages/feed/feed.dart';
import 'package:stooper_mobile/pages/map.dart';
import 'package:stooper_mobile/pages/user/home.dart';
import 'package:stooper_mobile/pages/messages.dart';
import 'package:stooper_mobile/providers/user.dart';
import 'package:stooper_mobile/services/user.dart';
import 'package:stooper_mobile/pages/dev/dev.dart';
import 'package:stooper_mobile/pages/loading/loading.dart';
import 'package:stooper_mobile/pages/prefs/prefs.dart';

Future<void> main() async {
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
      windowManager.setAlwaysOnTop(true);
    }
  }

  runApp(MultiProvider(
    providers: [
      ChangeNotifierProvider(
          create: (context) => UserProvider(store: UserStore())),
      ChangeNotifierProvider(create: (context) => LocationProvider()),
    ],
    child: const StooperMobileApp(),
  ));
}

class StooperMobileApp extends StatefulWidget {
  const StooperMobileApp({super.key});

  @override
  State<StooperMobileApp> createState() => _StooperMobileAppState();
}

class _StooperMobileAppState extends State<StooperMobileApp> {
  final _rootNavigator = GlobalKey<NavigatorState>();

  int? _currentPageIndex = 0;

  @override
  void initState() {
    super.initState();
    Provider.of<LocationProvider>(context, listen: false)
        .fetchCurrentLocation();
  }

  @override
  void dispose() {
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Stooper',
      theme: ThemeData(
        brightness: Brightness.light,
        primarySwatch: Colors.blue,
      ),
      darkTheme: ThemeData(
        brightness: Brightness.dark,
      ),
      themeMode: ThemeMode.dark,
      home: Scaffold(
        body: Navigator(
          key: _rootNavigator,
          initialRoute: '/',
          onGenerateRoute: (route) {
            Widget nextPage;
            int? nextPageIndex = _currentPageIndex;

            switch (route.name) {
              case '/':
                nextPage = const LoadingScreen();
                nextPageIndex = null;
                break;
              case '/feed':
                nextPage = const FeedScreen();
                nextPageIndex = 0;
                break;
              case '/map':
                nextPage = const MapScreen();
                nextPageIndex = 1;
                break;
              case '/messages':
                nextPage = const InboxScreen();
                nextPageIndex = 2;
                break;
              case '/home':
                nextPage = const HomeScreen();
                nextPageIndex = 3;
                break;
              case '/dev':
                nextPage = const DevScreen();
                nextPageIndex = 4;
                break;
              case '/prefs':
                nextPage = const PrefsScreen();
                nextPageIndex = 5;
                break;
              default:
                nextPage = const Center(
                  child: Text(
                    'Error: Unknown route',
                    style: TextStyle(color: Colors.white),
                  ),
                );
                nextPageIndex = 0;
            }

            if (nextPageIndex != _currentPageIndex) {
              WidgetsBinding.instance.addPostFrameCallback((_) {
                setState(() {
                  _currentPageIndex = nextPageIndex;
                });
              });
            }

            return MaterialPageRoute(
              settings: route,
              builder: (context) => nextPage,
            );
          },
        ),
        // drawer: Drawer(
        //   child: // Populate the Drawer in the next step.
        // ),
        bottomNavigationBar: _currentPageIndex != null
            ? NavigationBar(
                selectedIndex: _currentPageIndex!,
                onDestinationSelected: (int index) {
                  NavigatorState? navigatorState = _rootNavigator.currentState;

                  if (navigatorState == null) {
                    throw 'Root navigator not found!';
                  }

                  switch (index) {
                    case 0:
                      navigatorState.pushNamed('/feed');
                      break;
                    case 1:
                      navigatorState.pushNamed('/map');
                      break;
                    case 2:
                      navigatorState.pushNamed('/messages');
                      break;
                    case 3:
                      navigatorState.pushNamed('/home');
                      break;
                    case 4:
                      navigatorState.pushNamed('/dev');
                      break;
                    case 5:
                      navigatorState.pushNamed('/prefs');
                      break;
                  }
                },
                destinations: const [
                  NavigationDestination(
                    icon: Badge(
                      child: Icon(PhosphorIconsLight.cardsThree),
                    ),
                    selectedIcon: Badge(
                      label: Text("2"),
                      child: Icon(PhosphorIconsRegular.cardsThree),
                    ),
                    label: "Feed",
                  ),
                  NavigationDestination(
                    icon: Badge(
                      child: Icon(PhosphorIconsLight.mapTrifold),
                    ),
                    selectedIcon: Badge(
                      label: Text("2"),
                      child: Icon(PhosphorIconsRegular.mapTrifold),
                    ),
                    label: "Wander",
                  ),
                  NavigationDestination(
                    icon: Badge(
                      child: Icon(PhosphorIconsLight.mailbox),
                    ),
                    selectedIcon: Badge(
                      label: Text("2"),
                      child: Icon(PhosphorIconsRegular.mailbox),
                    ),
                    label: "Postbox",
                  ),
                  NavigationDestination(
                    icon: Icon(PhosphorIconsLight.barn),
                    selectedIcon: Icon(PhosphorIconsRegular.barn),
                    label: "Home",
                  ),
                  NavigationDestination(
                    icon: Icon(PhosphorIconsLight.terminalWindow),
                    selectedIcon: Icon(PhosphorIconsRegular.terminalWindow),
                    label: "Dev",
                  ),
                  NavigationDestination(
                    icon: Icon(PhosphorIconsLight.flower),
                    selectedIcon: Icon(PhosphorIconsRegular.flower),
                    label: "Prefs",
                  ),
                ],
              )
            : null,
      ),
    );
  }
}
