import "package:flutter/foundation.dart";
import "package:flutter/material.dart";

import "package:phosphor_flutter/phosphor_flutter.dart";

import "package:stooper_mobile/pages/feed/filters.dart";
import "package:stooper_mobile/pages/feed/index.dart";

class FeedScreen extends StatefulWidget {
  const FeedScreen({super.key});
  
  @override
  State<FeedScreen> createState() => _FeedScreenState();
}

class _FeedScreenState extends State<FeedScreen> {
  late TextEditingController _searchController;
  late FocusNode _searchFocusNode;
  bool _focused = false;

  @override
  void initState() {
    super.initState();
    _searchController = TextEditingController(text: '');
    _searchFocusNode = FocusNode(debugLabel: 'Search');
    _searchFocusNode.addListener(_handleFocusChange);
  }

  @override
  void dispose() {
    // Dispose the controller when the widget is removed from the tree
    _searchController.dispose();
    _searchFocusNode.dispose();
    super.dispose();
  }

  void _handleFocusChange() async {
    if (_searchFocusNode.hasFocus != _focused) {
      setState(() {
        _focused = _searchFocusNode.hasFocus;
      });
    }
  }

  void _showFilterOptions(BuildContext context) async {
    final result = await showDialog(
      context: context,
      builder: (BuildContext context) {
        return const FilterOverlayDialog();
      },
    );

    if (result != null) {
      // Handle the result from the dialog (e.g., apply filters to your feed)
      if (kDebugMode) {
        print('Filter options applied: $result');
      }
      // You would typically update the state of FeedScreen or trigger a data refresh
      // based on the selected filter options here.
    }
  }
  
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          icon: const Icon(PhosphorIconsLight.armchair),
          onPressed: () {
            //..
          },
        ),
        title: TextField(
          controller: _searchController,
          focusNode: _searchFocusNode,
          decoration: const InputDecoration(
            hintText: 'What\'s on your mind?',
            border: OutlineInputBorder(
              borderSide: BorderSide.none,
            ),
            contentPadding: EdgeInsets.zero,
          ),
          style: const TextStyle(fontSize: 16.0),
          onChanged: (text) {
            if (kDebugMode) {
              print('Search query: $text');
            }
          },
          onTap: () {
            if (kDebugMode) {
              print('Tapped!');
            }
          },
          onTapOutside: (pointerEvent) {
            if (kDebugMode) {
              print('Tapped: $pointerEvent');
            }
          },
          onSubmitted: (text) {
            if (kDebugMode) {
              print('Submitted search: $text');
            }
          },
        ),
        actions: [
          // IconButton(
          //   icon: const Icon(PhosphorIconsLight.x),
          //   onPressed: () {
          //     _searchController.clear();
          //     print('Clearing search field ..');
          //   },
          // ),
          IconButton(
            icon: const Icon(PhosphorIconsLight.listChecks),
            onPressed: () {
              _showFilterOptions(context);
            },
          ),
          IconButton(
            icon: const Icon(PhosphorIconsLight.at),
            onPressed: () {
              if (kDebugMode) {
                print('@ button pressed with text: ${_searchController.text}');
              }
            },
          ),
        ],
      ),
      body: Stack(
        children: [
          const FeedIndex(),
          if (_focused)
            Positioned.fill(
              child: Container(
                color: Colors.black.withValues(alpha: 0.8),
              ),
            ),
        ],
      ),
    );
  }
}
