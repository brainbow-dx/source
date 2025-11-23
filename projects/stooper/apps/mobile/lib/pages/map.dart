import "package:flutter/material.dart";

import "package:provider/provider.dart";

import "package:geolocator/geolocator.dart";

import "package:latlong2/latlong.dart";

import "package:flutter_map/flutter_map.dart";

// import "package:stooper_mobile/providers/user.dart";
import "package:stooper_mobile/providers/location.dart";

class MapScreen extends StatefulWidget {
  const MapScreen({super.key});

  @override
  State<MapScreen> createState() => _MapScreenState();
}

class _MapScreenState extends State<MapScreen> {
  final controller = MapController();

  // ignore: unused_field
  dynamic _style;

  @override
  void initState() {
    super.initState();

    _loadMapStyles();

    Provider.of<LocationProvider>(context, listen: false)
        .fetchCurrentLocation();
  }

  Future _loadMapStyles() async {
    // _style = await rootBundle.loadString('assets/json/dark_mode_style.json');
  }

  @override
  Widget build(BuildContext context) {
    final locationProvider = context.watch<LocationProvider>();
    Position? lastKnownLocation = locationProvider.lastKnownLocation;

    LatLng? currentLocation = lastKnownLocation != null
        ? LatLng(lastKnownLocation.latitude, lastKnownLocation.longitude)
        : null; // TODO: Fall-back to the user's neighborhood (zip?).

    const tileLayerTemplate = "https://tile.openstreetmap.org/{z}/{x}/{y}.png";
    // const tileLayerTemplate =
    //     "https://{s}.tile-cyclosm.openstreetmap.fr/cyclosm/{z}/{x}/{y}.png";

    return Scaffold(
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            Flexible(
              flex: 1,
              child: FlutterMap(
                mapController: controller,
                options: MapOptions(
                  center: currentLocation!,
                ),
                children: [
                  TileLayer(
                    userAgentPackageName: "com.mobile.stooper",
                    urlTemplate: tileLayerTemplate,
                    subdomains: const ["a", "b", "c"],
                    backgroundColor: const Color(0x22222200),
                  ),
                  const MarkerLayer(
                    markers: [
                      // Marker(
                      //   point: LatLng(30, 40),
                      //   width: 80,
                      //   height: 80,
                      //   child: FlutterLogo(),
                      // ),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
