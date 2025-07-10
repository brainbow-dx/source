class User {
  final String id;
  final String name;
  late String? email;

  User({
    required this.id,
    required this.name,
    this.email,
  }) {
    //..
  }

  factory User.fromJson(
    Map<String, dynamic> json,
  ) {
    return User(
      id: json['id'] as String,
      name: json['name'] as String,
      email: json['email'] as String?,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'email': email,
    };
  }
}
