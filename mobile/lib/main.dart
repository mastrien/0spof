import 'package:flutter/material.dart';

import 'screens.dart';
import 'src/rust/frb_generated.dart';
import 'store.dart';
import 'theme.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final store = Store();
  await store.init();
  runApp(SpofApp(store: store));
}

class SpofApp extends StatelessWidget {
  final Store store;
  const SpofApp({super.key, required this.store});

  @override
  Widget build(BuildContext context) => MaterialApp(
        title: '0spof',
        debugShowCheckedModeBanner: false,
        theme: buildTheme(Brightness.light),
        darkTheme: buildTheme(Brightness.dark),
        home: HomeShell(store: store),
      );
}
