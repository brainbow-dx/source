import 'dart:convert';
import 'package:http/http.dart' as http;
import '../models/user.dart';

class UserStore {
  final String baseUrl;

  UserStore({this.baseUrl = 'https://example.com/api'});

  Future<User?> getUser(String id) async {
    final uri = Uri.parse('$baseUrl/users/$id');
    final response = await http.get(uri);

    if (response.statusCode != 200) {
      throw Exception('Failed to load user: ${response.statusCode}');
    }

    return User.fromJson(jsonDecode(response.body));
  }

  Future<List<User>> getAllUsers() async {
    final response = await http.get(Uri.parse('$baseUrl/users'));

    if (response.statusCode != 200) {
      throw Exception('Failed to load users');
    }

    final jsonBody = jsonDecode(response.body);
    return jsonBody.map((json) => User.fromJson(json)).toList();
  }
}
