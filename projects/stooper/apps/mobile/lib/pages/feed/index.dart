import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'package:phosphor_flutter/phosphor_flutter.dart';

class FeedIndex extends StatelessWidget {
  const FeedIndex({
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: EdgeInsets.zero,
      child: ListView.separated(
        itemCount: _feedItems.length,
        padding: const EdgeInsets.symmetric(vertical: 15.0),
        physics: const BouncingScrollPhysics(),
        separatorBuilder: (BuildContext context, int index) {
          return const Column(
            children: [
              // TODO: Look for ads to show in the feed + display them.
              // TODO: Show divider only on a new day.
              Divider(
                height: 1,
                color: Color(0x22FFFFFF),
              ),
            ],
          );
        },
        itemBuilder: (BuildContext context, int index) {
          final item = _feedItems[index];
          return Center(
            child: GestureDetector(
              onTap: () {
                if (kDebugMode) {
                  print('Pressing container!');
                }
              },
              child: Container(
                constraints: const BoxConstraints(maxWidth: 600.0),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.start,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Padding(
                      padding: const EdgeInsets.all(12.0),
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.start,
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          // Avatar
                          Row(
                            children: [
                              Padding(
                                padding: const EdgeInsets.only(right: 8.0),
                                child: GestureDetector(
                                  onTap: () {
                                    if (kDebugMode) {
                                      print('Tapped avatar');
                                    }
                                  },
                                  child: AvatarImage(
                                    size: 42.0,
                                    url: item.user.imageUrl,
                                  ),
                                ),
                              ),
                              Expanded(
                                child: Column(
                                  mainAxisAlignment: MainAxisAlignment.start,
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    GestureDetector(
                                      onTap: () {
                                        if (kDebugMode) {
                                          print('TODO');
                                        }
                                      },
                                      child: Row(
                                        children: [
                                          RichText(
                                            overflow: TextOverflow.ellipsis,
                                            text: TextSpan(
                                              children: [
                                                TextSpan(
                                                  text: item.user.fullName,
                                                  style: theme
                                                      .textTheme.titleMedium,
                                                ),
                                                TextSpan(
                                                  text:
                                                      " @${item.user.userName}",
                                                  style: theme
                                                      .textTheme.titleMedium,
                                                ),
                                              ],
                                            ),
                                          ),
                                        ],
                                      ),
                                    ),
                                    Row(
                                      children: [
                                        RichText(
                                          overflow: TextOverflow.ellipsis,
                                          text: TextSpan(
                                            children: [
                                              TextSpan(
                                                text: "5m ago - 2.3 miles away",
                                                style:
                                                    theme.textTheme.titleSmall,
                                              ),
                                            ],
                                          ),
                                        ),
                                      ],
                                    ),
                                  ],
                                ),
                              ),
                              IconButton(
                                icon:
                                    const Icon(PhosphorIconsRegular.dotsThree),
                                onPressed: () {
                                  //..
                                },
                              ),
                            ],
                          ),
                          // Content
                          Column(
                            mainAxisAlignment: MainAxisAlignment.start,
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              if (item.title != null)
                                // Large Title Image
                                Container(
                                  margin: const EdgeInsets.only(
                                    top: 20.0,
                                    bottom: 10.0,
                                  ),
                                  child: RichText(
                                    overflow: TextOverflow.ellipsis,
                                    text: TextSpan(
                                      text: item.title,
                                      style: theme.textTheme.headlineSmall,
                                    ),
                                  ),
                                ),
                              if (item.content != null)
                                // Body Content
                                Container(
                                  margin: const EdgeInsets.only(
                                    top: 10.0,
                                    bottom: 10.0,
                                  ),
                                  child: RichText(
                                    text: TextSpan(
                                      text: item.content,
                                      style:
                                          Theme.of(context).textTheme.bodyLarge,
                                    ),
                                  ),
                                ),
                              if (item.imageUrl != null)

                                // Image Gallery
                                Container(
                                  margin: const EdgeInsets.only(
                                    top: 10.0,
                                    bottom: 10.0,
                                  ),
                                  child: Row(
                                    children: [
                                      Container(
                                        width: 80.0,
                                        height: 80.0,
                                        decoration: BoxDecoration(
                                          borderRadius:
                                              BorderRadius.circular(6.0),
                                          image: DecorationImage(
                                            fit: BoxFit.cover,
                                            image: NetworkImage(item.imageUrl!),
                                          ),
                                        ),
                                      ),
                                    ],
                                  ),
                                ),
                            ],
                          ),
                          FeedItemActionsBar(item: item),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}

class AvatarImage extends StatelessWidget {
  final double size;
  final String url;

  const AvatarImage({
    super.key,
    required this.size,
    required this.url,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        image: DecorationImage(image: NetworkImage(url)),
      ),
    );
  }
}

class FeedItemActionsBar extends StatelessWidget {
  final FeedItem item;

  const FeedItemActionsBar({
    super.key,
    required this.item,
  });

  @override
  Widget build(BuildContext context) {
    return Theme(
      data: Theme.of(context).copyWith(
        iconTheme: const IconThemeData(
          color: Colors.grey,
          size: 16,
        ),
        textButtonTheme: TextButtonThemeData(
          style: ButtonStyle(
            foregroundColor: WidgetStateProperty.all(Colors.grey),
          ),
        ),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.start,
        children: [
          TextButton.icon(
            icon: const Icon(PhosphorIconsBold.heart),
            label: Text(
              item.likesCount == 0 ? '' : item.likesCount.toString(),
            ),
            onPressed: () {
              //..
            },
          ),
          TextButton.icon(
            icon: const Icon(PhosphorIconsBold.note),
            label: Text(
              item.commentsCount == 0 ? '' : item.commentsCount.toString(),
            ),
            onPressed: () {
              //..
            },
          ),
          TextButton.icon(
            icon: const Icon(PhosphorIconsBold.envelopeSimple),
            label: Text(
              item.commentsCount == 0 ? '' : item.commentsCount.toString(),
            ),
            onPressed: () {
              //..
            },
          ),
          Flexible(
            fit: FlexFit.tight,
            child: Container(),
          ),
          TextButton.icon(
            icon: const Icon(PhosphorIconsBold.coins),
            // iconAlignment: IconAlignment.end,
            label: Text(
              item.shareCount == 0 ? 'tip?' : item.shareCount.toString(),
            ),
            onPressed: () {
              //..
            },
          ),
        ],
      ),
    );
  }
}

class FeedItem {
  final String? title;
  final String? content;
  final String? imageUrl;
  final User user;
  final int commentsCount;
  final int likesCount;
  final int shareCount;

  FeedItem({
    this.title,
    this.content,
    this.imageUrl,
    required this.user,
    this.commentsCount = 0,
    this.likesCount = 0,
    this.shareCount = 0,
  });
}

class User {
  final String fullName;
  final String imageUrl;
  final String userName;

  User(this.fullName, this.userName, this.imageUrl);
}

final List<User> _users = [
  User("John Doe", "john_doe", "https://picsum.photos/id/1062/80/80"),
  User("Jane Doe", "jane_doe", "https://picsum.photos/id/1066/80/80"),
  User("Jack Doe", "jack_doe", "https://picsum.photos/id/1072/80/80"),
  User("Jill Doe", "jill_doe", "https://picsum.photos/id/133/80/80"),
];

final List<FeedItem> _feedItems = [
  FeedItem(
    content:
        "A son asked his father (a programmer) why the sun rises in the east, and sets in the west. His response? It works, don’t touch!",
    user: _users[0],
    // imageUrl: "https://picsum.photos/id/1000/960/540",
    likesCount: 100,
    commentsCount: 10,
    shareCount: 1,
  ),
  FeedItem(
    user: _users[1],
    imageUrl: "https://picsum.photos/id/1001/960/540",
    likesCount: 10,
    commentsCount: 2,
  ),
  FeedItem(
    user: _users[0],
    content:
        "How many programmers does it take to change a light bulb? None, that’s a hardware problem.",
    likesCount: 50,
    commentsCount: 22,
    shareCount: 30,
  ),
  FeedItem(
    user: _users[1],
    content:
        "Programming today is a race between software engineers striving to build bigger and better idiot-proof programs, and the Universe trying to produce bigger and better idiots. So far, the Universe is winning.",
    imageUrl: "https://picsum.photos/id/1002/960/540",
    likesCount: 500,
    commentsCount: 202,
    shareCount: 120,
  ),
  FeedItem(
    user: _users[2],
    content: "Good morning!",
    imageUrl: "https://picsum.photos/id/1003/960/540",
  ),
  FeedItem(user: _users[1], imageUrl: "https://picsum.photos/id/1004/960/540"),
  FeedItem(user: _users[3], imageUrl: "https://picsum.photos/id/1005/960/540"),
  FeedItem(user: _users[0], imageUrl: "https://picsum.photos/id/1006/960/540"),
];
