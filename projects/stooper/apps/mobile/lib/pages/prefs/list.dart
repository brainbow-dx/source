import 'package:flutter/material.dart';

class FeedList extends StatelessWidget {
  const FeedList({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: EdgeInsets.zero,
      constraints: const BoxConstraints(maxWidth: 600.0),
      child: ListView.separated(
        itemCount: _feedItems.length,
        padding: EdgeInsets.zero,
        physics: const BouncingScrollPhysics(),
        separatorBuilder: (BuildContext context, int index) {
          return const Divider(
            height: 1,
          );
        },
        itemBuilder: (BuildContext context, int index) {
          final item = _feedItems[index];
          return Column(
            mainAxisAlignment: MainAxisAlignment.start,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Padding(
                padding: const EdgeInsets.all(12.0),
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.start,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Padding(
                          padding: const EdgeInsets.only(right: 8.0),
                          child: AvatarImage(
                            size: 36.0,
                            url: item.user.imageUrl,
                          ),
                        ),
                        Expanded(
                          child: RichText(
                            overflow: TextOverflow.ellipsis,
                            text: TextSpan(
                              children: [
                                TextSpan(
                                  text: item.user.fullName,
                                  style: const TextStyle(
                                    fontWeight: FontWeight.bold,
                                  ),
                                ),
                                TextSpan(
                                  text: " @${item.user.userName}",
                                  style: Theme.of(
                                    context,
                                  ).textTheme.bodySmall,
                                ),
                              ],
                            ),
                          ),
                        ),
                        Text(
                          '5m',
                          style: Theme.of(context).textTheme.bodySmall,
                        ),
                      ],
                    ),
                    if (item.content != null)
                      Container(
                        margin: const EdgeInsets.only(top: 8.0),
                        child: Text(item.content!),
                      ),
                    if (item.imageUrl != null)
                      Container(
                        height: 200,
                        margin: const EdgeInsets.only(top: 8.0),
                        decoration: BoxDecoration(
                          borderRadius: BorderRadius.circular(6.0),
                          image: DecorationImage(
                            fit: BoxFit.cover,
                            image: NetworkImage(item.imageUrl!),
                          ),
                        ),
                      ),
                    FeedItemActionsBar(item: item),
                  ],
                ),
              ),
            ],
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
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          TextButton.icon(
            icon: const Icon(Icons.favorite),
            label: Text(
              item.likesCount == 0 ? '' : item.likesCount.toString(),
              // textScaleFactor: 0.8,
            ),
            onPressed: () {
              //..
            },
          ),
          TextButton.icon(
            icon: const Icon(Icons.notes),
            label: Text(
              item.commentsCount == 0 ? '' : item.commentsCount.toString(),
              // textScaleFactor: 0.8,
            ),
            onPressed: () {
              //..
            },
          ),
          TextButton.icon(
            icon: const Icon(Icons.shortcut),
            label: Text(
              item.retweetsCount == 0 ? '' : item.retweetsCount.toString(),
              // textScaleFactor: 0.8,
            ),
            onPressed: () {
              //..
            },
          ),
          // TextButton.icon(
          //   icon: const Icon(CupertinoIcons.share_up),
          //   label: const Text(
          //     '',
          //     textScaleFactor: 0.8,
          //   ),
          //   onPressed: () {},
          // ),
          IconButton(
            icon: const Icon(Icons.send),
            highlightColor: null,
            onPressed: () {
              //..
            },
          ),
          IconButton(
            icon: const Icon(Icons.more_horiz),
            highlightColor: null,
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
  final String? content;
  final String? imageUrl;
  final User user;
  final int commentsCount;
  final int likesCount;
  final int retweetsCount;

  FeedItem({
    this.content,
    this.imageUrl,
    required this.user,
    this.commentsCount = 0,
    this.likesCount = 0,
    this.retweetsCount = 0,
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
    imageUrl: "https://picsum.photos/id/1000/960/540",
    likesCount: 100,
    commentsCount: 10,
    retweetsCount: 1,
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
    retweetsCount: 30,
  ),
  FeedItem(
    user: _users[1],
    content:
        "Programming today is a race between software engineers striving to build bigger and better idiot-proof programs, and the Universe trying to produce bigger and better idiots. So far, the Universe is winning.",
    imageUrl: "https://picsum.photos/id/1002/960/540",
    likesCount: 500,
    commentsCount: 202,
    retweetsCount: 120,
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
