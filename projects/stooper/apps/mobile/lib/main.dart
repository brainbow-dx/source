import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:stooper_mobile/providers/location.dart';

import 'package:window_manager/window_manager.dart';

import 'package:logging/logging.dart';

import 'package:provider/provider.dart';

import 'package:stooper_mobile/pages/feed/feed.dart';
import 'package:stooper_mobile/pages/map.dart';
import 'package:stooper_mobile/pages/user.dart';
import 'package:stooper_mobile/pages/messages.dart';
import 'package:stooper_mobile/providers/user.dart';
import 'package:stooper_mobile/services/user.dart';

Future<void> main() async {
  Logger.root.level = Level.ALL; // defaults to Level.INFO
  Logger.root.onRecord.listen((record) {
    if (kDebugMode) {
      print('${record.level.name}: ${record.time}: ${record.message}');
    }
  });

  WidgetsFlutterBinding.ensureInitialized();
  await windowManager.ensureInitialized();

  windowManager.setAlwaysOnTop(true);

  runApp(MultiProvider(
    providers: [
      ChangeNotifierProvider(
        create: (context) => UserProvider(store: UserStore()),
      ),
      ChangeNotifierProvider(
        create: (context) => LocationProvider(),
      ),
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

  int _currentPageIndex = 0;

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
      title: 'Stooper Mobile',
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
          onGenerateRoute: (settings) {
            Widget nextPage;
            int nextPageIndex = _currentPageIndex;

            switch (settings.name) {
              case '/':
                nextPage = const FeedScreen();
                nextPageIndex = 0;
                break;
              case '/map':
                nextPage = const MapScreen();
                nextPageIndex = 1;
                break;
              case '/inbox':
                nextPage = const InboxScreen();
                nextPageIndex = 2;
                break;
              case '/user':
                nextPage = const UserScreen();
                nextPageIndex = 3;
                break;
              default:
                nextPage = const Center(
                  child: Text(
                    'Error: Unknown route',
                    style: TextStyle(color: Colors.white),
                  ),
                );
                nextPageIndex = -1;
            }

            if (nextPageIndex != _currentPageIndex) {
              WidgetsBinding.instance.addPostFrameCallback((_) {
                setState(() {
                  _currentPageIndex = nextPageIndex;
                });
              });
            }

            return MaterialPageRoute(
              settings: settings,
              builder: (context) => nextPage,
            );
          },
        ),
        // drawer: Drawer(
        //   child: // Populate the Drawer in the next step.
        // ),
        bottomNavigationBar: NavigationBar(
          selectedIndex: _currentPageIndex,
          // indicatorColor: Colors.amber,
          onDestinationSelected: (int index) {
            NavigatorState? rootNavigatorState = _rootNavigator.currentState;

            if (rootNavigatorState == null) {
              throw 'Root navigator not found!';
            }

            switch (index) {
              case 0:
                rootNavigatorState.pushNamed('/');
                break;
              case 1:
                rootNavigatorState.pushNamed('/map');
                break;
              case 2:
                rootNavigatorState.pushNamed('/inbox');
                break;
              case 3:
                rootNavigatorState.pushNamed('/user');
                break;
            }
          },
          destinations: const [
            NavigationDestination(
              label: "Feed",
              icon: Icon(Icons.storefront),
            ),
            NavigationDestination(
              label: "Map",
              icon: Badge(
                child: Icon(Icons.map_outlined),
              ),
            ),
            NavigationDestination(
              label: "Inbox",
              icon: Badge(
                label: Text("2"),
                child: Icon(Icons.messenger),
              ),
            ),
            NavigationDestination(
              label: "User",
              icon: Icon(Icons.person),
            ),
          ],
        ),
      ),
    );
  }
}
