import 'package:flutter/foundation.dart';

import 'package:uuid/v4.dart';

import 'package:stooper_mobile/models/user.dart';
import 'package:stooper_mobile/services/user.dart';

// providers/user_provider.dart
class UserProvider extends ChangeNotifier {
  final Map<String, User> _users = {
    'TODO': User(id: 'TODO', name: 'moodring'),
  };

  final UserStore _userStore;

  UserProvider({
    required UserStore store,
  }) : _userStore = store {
    //..
  }

  User? getUser(String id) {
    return _users[id];
  }

  /// Save a user record and persist it to the backend store.
  ///
  /// @returns the previously stored user, if any.
  ///
  User? insertUser(User user) {
    User? prevUser = getUser(user.id);
    _users[user.id] = user;
    notifyListeners();
    return prevUser;
  }

  void clear() {
    _users.clear();
    notifyListeners();
  }

  Future<String> generateUserID() async {
    return const UuidV4().toString();
  }

  Future<void> fetchUser(String id) async {
    try {
      final user = await _userStore.getUser(id);
      if (user != null) {
        insertUser(user);
      }
    } catch (exc) {
      if (kDebugMode) {
        print('Failed to get user: $exc');
      }
    }
  }
}
