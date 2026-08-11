import 'package:flutter/material.dart';

class FilterOverlayDialog extends StatefulWidget {
  const FilterOverlayDialog({super.key});

  @override
  State<FilterOverlayDialog> createState() => _FilterOverlayDialogState();
}

class _FilterOverlayDialogState extends State<FilterOverlayDialog> {
  // Example filter options (you'd replace these with your actual options)
  bool _showOnlyMyPosts = false;
  String? _selectedCategory;
  final List<String> _categories = ['Technology', 'Science', 'Art', 'Sports'];

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Filter Options'),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              children: [
                Checkbox(
                  value: _showOnlyMyPosts,
                  onChanged: (bool? value) {
                    setState(() {
                      _showOnlyMyPosts = value ?? false;
                    });
                  },
                ),
                const Text('Show only my posts'),
              ],
            ),
            DropdownButtonFormField<String>(
              initialValue: _selectedCategory,
              hint: const Text('Select Category'),
              items: _categories.map((category) {
                return DropdownMenuItem(
                  value: category,
                  child: Text(category),
                );
              }).toList(),
              onChanged: (String? newValue) {
                setState(() {
                  _selectedCategory = newValue;
                });
              },
              decoration: const InputDecoration(
                border: OutlineInputBorder(),
                contentPadding:
                    EdgeInsets.symmetric(horizontal: 10, vertical: 5),
              ),
            ),
            const SizedBox(height: 16),
            // Add more filter options here (e.g., date pickers, range sliders)
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () {
            Navigator.of(context).pop(); // Close the dialog without applying
          },
          child: const Text('Cancel'),
        ),
        ElevatedButton(
          onPressed: () {
            // Apply filters and pass them back (or trigger a callback)
            Navigator.of(context).pop({
              'showOnlyMyPosts': _showOnlyMyPosts,
              'selectedCategory': _selectedCategory,
            });
          },
          child: const Text('Apply Filters'),
        ),
      ],
    );
  }
}
