import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

typedef _NewNative = Pointer<Void> Function();
typedef _FreeNative = Void Function(Pointer<Void>);
typedef _EvalNative =
    Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>, Bool);
typedef _SheetNative = Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>);
typedef _AtNative = Double Function(Pointer<Void>, Pointer<Utf8>, Double);
typedef _AngleNative = Void Function(Pointer<Void>, Uint8);
typedef _StringFreeNative = Void Function(Pointer<Utf8>);
typedef _LoanNative = Double Function(Double, Double, Double);

class CalcReply {
  const CalcReply(this.ok, this.text);
  final bool ok;
  final String text;
}

class SheetLine {
  const SheetLine(this.kind, this.text);
  final String kind;
  final String text;
}

class CalculatorCore {
  CalculatorCore()
    : _lib = DynamicLibrary.open(
        Platform.isAndroid
            ? 'libscicalc_mobile_core.so'
            : 'native/target/debug/libscicalc_mobile_core.dylib',
      ) {
    _new = _lib.lookupFunction<_NewNative, _NewNative>('scicalc_new');
    _free = _lib.lookupFunction<_FreeNative, void Function(Pointer<Void>)>(
      'scicalc_free',
    );
    _eval = _lib
        .lookupFunction<
          _EvalNative,
          Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>, bool)
        >('scicalc_eval');
    _sheet = _lib
        .lookupFunction<
          _SheetNative,
          Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>)
        >('scicalc_sheet');
    _at = _lib
        .lookupFunction<
          _AtNative,
          double Function(Pointer<Void>, Pointer<Utf8>, double)
        >('scicalc_eval_at');
    _angle = _lib
        .lookupFunction<_AngleNative, void Function(Pointer<Void>, int)>(
          'scicalc_set_angle',
        );
    _stringFree = _lib
        .lookupFunction<_StringFreeNative, void Function(Pointer<Utf8>)>(
          'scicalc_string_free',
        );
    _loan = _lib
        .lookupFunction<_LoanNative, double Function(double, double, double)>(
          'scicalc_loan_payment',
        );
    _env = _new();
    if (_env == nullptr) throw StateError('Could not start calculator');
  }

  final DynamicLibrary _lib;
  late final _NewNative _new;
  late final void Function(Pointer<Void>) _free;
  late final Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>, bool) _eval;
  late final Pointer<Utf8> Function(Pointer<Void>, Pointer<Utf8>) _sheet;
  late final double Function(Pointer<Void>, Pointer<Utf8>, double) _at;
  late final void Function(Pointer<Void>, int) _angle;
  late final void Function(Pointer<Utf8>) _stringFree;
  late final double Function(double, double, double) _loan;
  late final Pointer<Void> _env;
  bool _closed = false;

  Map<String, dynamic> _read(Pointer<Utf8> pointer) {
    try {
      return jsonDecode(pointer.toDartString()) as Map<String, dynamic>;
    } finally {
      _stringFree(pointer);
    }
  }

  CalcReply calculate(String expression, {bool commit = false}) {
    final text = expression.toNativeUtf8();
    try {
      final reply = _read(_eval(_env, text, commit));
      return CalcReply(
        reply['ok'] == true,
        (reply['ok'] == true ? reply['value'] : reply['error']) as String,
      );
    } finally {
      calloc.free(text);
    }
  }

  List<SheetLine> runSheet(String source) {
    final text = source.toNativeUtf8();
    try {
      final reply = _read(_sheet(_env, text));
      if (reply['ok'] != true) throw FormatException(reply['error'] as String);
      return (reply['lines'] as List<dynamic>)
          .map(
            (line) => SheetLine(
              line['kind'] as String,
              (line['value'] as String?) ?? '',
            ),
          )
          .toList();
    } finally {
      calloc.free(text);
    }
  }

  double at(String expression, double x) {
    final text = expression.toNativeUtf8();
    try {
      return _at(_env, text, x);
    } finally {
      calloc.free(text);
    }
  }

  void setAngle(int value) => _angle(_env, value);

  double loanPayment(double principal, double rate, double months) =>
      _loan(principal, rate, months);

  void dispose() {
    if (!_closed) {
      _closed = true;
      _free(_env);
    }
  }
}
