import 'package:flutter_test/flutter_test.dart';
import 'package:scientific_calculator_studio_pro/tex_to_engine.dart';

void main() {
  test('touch math templates become expressions for the shared engine', () {
    final scientific = texToEngine(r'\sin(30)+\sqrt{9}');
    expect(scientific, 'sin(30)+sqrt(9)');
    final fraction = texToEngine(r'\frac{1}{2\pi*4700*100n}');
    expect(fraction, '((1)/(2π*4700*100n))');
    expect(texToEngine(r'x^{2}'), 'x^(2)');
    expect(texToEngine(r'240\cdot18\%'), '240*18%');
    expect(texToEngine(r'\sqrt[3]{8}'), '((8)^(1/(3)))');
    expect(() => texToEngine(r'\frac{1}{\Box}'), throwsFormatException);
  });
}
