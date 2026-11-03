/// Turns the touch editor's TeX into the desktop engine's expression syntax.
/// Users never see or type these commands.
String texToEngine(String tex) => _Reader(tex).read();

class _Reader {
  _Reader(this.source);
  final String source;
  int index = 0;

  String read([String? until]) {
    final out = StringBuffer();
    while (index < source.length) {
      final char = source[index++];
      if (char == until) return out.toString();
      if (char == r'\') {
        final start = index;
        while (index < source.length &&
            RegExp(r'[A-Za-z]').hasMatch(source[index])) {
          index++;
        }
        final command = source.substring(start, index);
        switch (command) {
          case 'frac':
            final numerator = group('{', '}');
            final denominator = group('{', '}');
            out.write('(($numerator)/($denominator))');
          case 'sqrt':
            final degree = peek('[') ? group('[', ']') : null;
            final radicand = group('{', '}');
            out.write(
              degree == null
                  ? 'sqrt($radicand)'
                  : '(($radicand)^(1/($degree)))',
            );
          case 'Box':
            throw const FormatException('Complete every math slot');
          case 'pi':
            out.write('π');
          case 'cdot':
          case 'times':
            out.write('*');
          case 'div':
            out.write('/');
          case '':
            if (index < source.length && source[index] == '%') {
              index++;
              out.write('%');
            }
          case 'left':
          case 'right':
          case ',':
          case ';':
            break;
          case 'theta':
            out.write('θ');
          case 'alpha':
            out.write('α');
          case 'beta':
            out.write('β');
          case 'Omega':
            out.write('Ω');
          default:
            if (command.isNotEmpty) {
              out.write(command);
            }
        }
      } else if (char == '^') {
        out.write('^(${peek('{') ? group('{', '}') : source[index++]})');
      } else if (char == '_') {
        out.write(peek('{') ? group('{', '}') : source[index++]);
      } else if (char == '{') {
        out.write('(${read('}')})');
      } else if (char == '}' || char == ']') {
        throw const FormatException('Unmatched math bracket');
      } else if (char.trim().isNotEmpty) {
        out.write(char == '−' ? '-' : char);
      }
    }
    if (until != null) throw const FormatException('Unclosed math structure');
    return out.toString();
  }

  bool peek(String char) => index < source.length && source[index] == char;

  String group(String open, String close) {
    if (!peek(open)) throw FormatException('Expected $open');
    index++;
    return read(close);
  }
}
