import 'package:flutter_test/flutter_test.dart';
import 'package:spof_mobile_app/updater.dart';

void main() {
  test('isNewer compara versões semânticas', () {
    expect(isNewer('0.3.0', '0.2.0'), isTrue);
    expect(isNewer('0.10.0', '0.9.3'), isTrue);
    expect(isNewer('0.2.0', '0.2.0'), isFalse);
    expect(isNewer('0.2', '0.2.1'), isFalse);
    expect(isNewer('1.0.0-beta', '0.9.9'), isTrue);
  });
}
