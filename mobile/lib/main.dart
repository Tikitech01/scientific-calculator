import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_math_fork/flutter_math.dart' as fm;
import 'package:math_keyboard/math_keyboard.dart';
// math_keyboard exposes its editor controller but keeps TeXArg in src.
// ignore: implementation_imports
import 'package:math_keyboard/src/foundation/node.dart' show TeXArg;
import 'package:shared_preferences/shared_preferences.dart';

import 'calculator_core.dart';
import 'tex_to_engine.dart';

const canvas = Color(0xFF121214);
const surface = Color(0xFF1B1B1E);
const keyColor = Color(0xFF222226);
const border = Color(0xFF2B2B30);
const muted = Color(0xFF8E8E96);
const orange = Color(0xFFFF6A3D);
const foreground = Color(0xFFEDEDEB);

void main() => runApp(const CalculatorApp());

class CalculatorApp extends StatelessWidget {
  const CalculatorApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'Scientific Calculator Studio Pro',
    debugShowCheckedModeBanner: false,
    theme: ThemeData(
      useMaterial3: true,
      brightness: Brightness.dark,
      scaffoldBackgroundColor: canvas,
      colorScheme: const ColorScheme.dark(
        primary: orange,
        secondary: orange,
        surface: surface,
        onSurface: foreground,
      ),
    ),
    home: const CalculatorScreen(),
  );
}

enum CalcView { simple, semi, pro }

enum MathPage { functions, symbols, structures }

const toolNames = [
  'Graph',
  'Sheet',
  'Matrix',
  'Equation',
  'Statistics',
  'Finance',
  'Convert',
  'Programmer',
];
const shortcutNames = ['sin', '√', 'π', 'cos', 'log', 'x²'];
const sheetExample = 'R = 4700\nC = 100n\n1/(2*pi*R*C)';
const advancedFunctionExamples = <String, List<String>>{
  'Derivative': ['Equation', 'diff(x^2,3)'],
  'Integral': ['Equation', 'integ(x^2,0,1)'],
  'Limit': ['Equation', 'limit(sin(x)/x,0)'],
  'Summation': ['Equation', 'sum(k^2,k,1,10)'],
  'Eigenvalues': ['Matrix', 'eig([[1,2],[3,4]])'],
  'Standard deviation': ['Statistics', 'stdev({1,2,3,4})'],
  'Binomial probability': ['Statistics', 'binompdf(10,3,0.5)'],
  'Unit conversion': ['Convert', '5 km to m'],
  'Bitwise AND': ['Programmer', 'and(12,10)'],
  'Loan payment': ['Finance', ''],
};

class HistoryItem {
  const HistoryItem(this.tex, this.result);
  final String tex;
  final String result;
  Map<String, String> toJson() => {'tex': tex, 'result': result};
}

class CalculatorScreen extends StatefulWidget {
  const CalculatorScreen({super.key});
  @override
  State<CalculatorScreen> createState() => _CalculatorScreenState();
}

class _CalculatorScreenState extends State<CalculatorScreen> {
  late final CalculatorCore core;
  late final MathFieldEditingController editor;
  final prefs = SharedPreferencesAsync();
  final sheetText = TextEditingController();
  final toolText = TextEditingController();
  final financePrincipal = TextEditingController(text: '200000');
  final financeRate = TextEditingController(text: '9');
  final financeMonths = TextEditingController(text: '240');
  CalcView view = CalcView.simple;
  CalcView defaultView = CalcView.simple;
  MathPage mathPage = MathPage.structures;
  bool mathOpen = false, justEvaluated = false, adaptive = true;
  String tex = '', result = '0';
  String? error, workspace, toolResult;
  int angle = 0;
  final usage = <String, int>{};
  final pinned = <String>{}, hidden = <String>{};
  final history = <HistoryItem>[];
  List<SheetLine> sheetLines = [];
  double graphRange = 10;
  double graphCenter = 0;

  @override
  void initState() {
    super.initState();
    core = CalculatorCore();
    editor = MathFieldEditingController()..addListener(_expressionChanged);
    unawaited(_load());
  }

  Future<void> _load() async {
    final saved = await prefs.getString('defaultView');
    final use = await prefs.getString('shortcutUse');
    final pins = await prefs.getStringList('shortcutPins');
    final hides = await prefs.getStringList('shortcutHides');
    final adaptiveSaved = await prefs.getBool('adaptiveShortcuts');
    final historySaved = await prefs.getString('history');
    final sheetSaved = await prefs.getString('sheet');
    if (!mounted) return;
    setState(() {
      defaultView = CalcView.values.firstWhere(
        (v) => v.name == saved,
        orElse: () => CalcView.simple,
      );
      view = defaultView;
      if (use != null) {
        for (final entry in (jsonDecode(use) as Map<String, dynamic>).entries) {
          usage[entry.key] = entry.value as int;
        }
      }
      pinned.addAll(pins ?? []);
      hidden.addAll(hides ?? []);
      adaptive = adaptiveSaved ?? true;
      if (historySaved != null) {
        for (final item in jsonDecode(historySaved) as List<dynamic>) {
          history.add(
            HistoryItem(item['tex'] as String, item['result'] as String),
          );
        }
      }
      sheetText.text = sheetSaved ?? '';
    });
  }

  void _saveShortcuts() {
    unawaited(prefs.setString('shortcutUse', jsonEncode(usage)));
    unawaited(prefs.setStringList('shortcutPins', pinned.toList()));
    unawaited(prefs.setStringList('shortcutHides', hidden.toList()));
    unawaited(prefs.setBool('adaptiveShortcuts', adaptive));
  }

  void _used(String name) {
    usage[name] = (usage[name] ?? 0) + 1;
    _saveShortcuts();
  }

  List<String> get shortcuts {
    final items = shortcutNames
        .where((name) => !hidden.contains(name))
        .toList();
    items.sort((a, b) {
      final pin = (pinned.contains(b) ? 1 : 0) - (pinned.contains(a) ? 1 : 0);
      if (pin != 0) return pin;
      final count = adaptive ? (usage[b] ?? 0).compareTo(usage[a] ?? 0) : 0;
      return count != 0
          ? count
          : shortcutNames.indexOf(a).compareTo(shortcutNames.indexOf(b));
    });
    return items.take(3).toList();
  }

  String get engineExpression {
    try {
      return texToEngine(tex);
    } on FormatException {
      return '';
    }
  }

  void _expressionChanged() {
    final current = editor.currentEditingValue(placeholderWhenEmpty: false);
    var next = current.isEmpty ? '0' : result;
    if (current.isNotEmpty) {
      try {
        final reply = core.calculate(texToEngine(current));
        if (reply.ok) next = reply.text;
      } on FormatException {
        /* An unfinished template keeps the last preview. */
      }
    }
    if (!mounted) return;
    setState(() {
      tex = current;
      result = next;
      error = null;
    });
  }

  void _leaf(String value, {String? used}) {
    if (justEvaluated && RegExp(r'^[0-9.]$').hasMatch(value)) editor.clear();
    justEvaluated = false;
    editor.addLeaf(value);
    if (used != null) _used(used);
  }

  void _numberKey(String label) {
    if (label == 'AC') {
      editor.clear();
      setState(() {
        result = '0';
        error = null;
        justEvaluated = false;
      });
    } else if (label == '±') {
      if (view == CalcView.simple && tex.isNotEmpty) {
        final value = tex.startsWith('-') ? tex.substring(1) : '-$tex';
        editor.clear();
        for (final rune in value.runes) {
          editor.addLeaf(String.fromCharCode(rune));
        }
      } else {
        _leaf('-');
      }
    } else if (label == '%') {
      _leaf(r'\%');
    } else if (label == '=') {
      _commit();
    } else {
      _leaf(switch (label) {
        '×' => r'\cdot',
        '÷' => r'\div',
        '−' => '-',
        _ => label,
      });
    }
  }

  void _commit() {
    if (tex.isEmpty) return;
    try {
      final reply = core.calculate(texToEngine(tex), commit: true);
      setState(() {
        if (reply.ok) {
          result = reply.text;
          error = null;
          history.insert(0, HistoryItem(tex, result));
          if (history.length > 50) history.removeLast();
          unawaited(
            prefs.setString(
              'history',
              jsonEncode(history.map((h) => h.toJson()).toList()),
            ),
          );
        } else {
          error = reply.text;
        }
        justEvaluated = reply.ok;
      });
    } on FormatException catch (e) {
      setState(() => error = e.message);
    }
  }

  void _shortcut(String name) {
    if (['sin', 'cos', 'log'].contains(name)) {
      _leaf('\\$name(', used: name);
    } else if (name == '√') {
      editor.addFunction(r'\sqrt', [TeXArg.braces]);
      _used(name);
    } else if (name == 'π') {
      _leaf(r'\pi', used: name);
    } else if (name == 'x²') {
      editor.addFunction('^2', [TeXArg.braces]);
      _used(name);
    }
  }

  void _mathKey(String label) {
    switch (label) {
      case '□/□':
        editor.addFunction(r'\frac', [TeXArg.braces, TeXArg.braces]);
        break;
      case 'x□':
        editor.addFunction('^', [TeXArg.braces]);
        break;
      case '√□':
      case '√':
        editor.addFunction(r'\sqrt', [TeXArg.braces]);
        break;
      case 'ⁿ√□':
        editor.addFunction(r'\sqrt', [TeXArg.brackets, TeXArg.braces]);
        break;
      case '|x|':
        _leaf('abs(');
        break;
      case '(□)':
        editor.addFunction('', [TeXArg.parentheses]);
        break;
      case 'x²':
        editor.addFunction('^2', [TeXArg.braces]);
        break;
      case 'x⁻¹':
        editor.addFunction('^-1', [TeXArg.braces]);
        break;
      case 'eˣ':
        _leaf('e');
        editor.addFunction('^', [TeXArg.braces]);
        break;
      case 'n!':
        _leaf('!');
        break;
      case 'mod':
        _leaf('mod(');
        break;
      case 'Ans':
        _leaf('Ans');
        break;
      case 'π':
        _leaf(r'\pi');
        break;
      case 'sin':
      case 'cos':
      case 'tan':
      case 'ln':
      case 'log':
        _leaf('\\$label(');
        break;
      default:
        _leaf(label);
    }
    _used(label);
  }

  void _openWorkspace(String name, {String? example}) {
    _used(name);
    if (name == 'Sheet' &&
        sheetText.text.trim().isEmpty &&
        engineExpression.isNotEmpty) {
      sheetText.text = engineExpression;
    }
    if (name != 'Graph' && name != 'Sheet') {
      toolText.text =
          example ??
          switch (name) {
            'Matrix' => 'det([[1,2],[3,4]])',
            'Equation' => 'solve(x^2-2,1)',
            'Statistics' => 'mean({1,2,3,4})',
            'Convert' => '5 km to m',
            'Programmer' => 'and(12,10)',
            _ => '',
          };
      toolResult = null;
    }
    setState(() => workspace = name);
  }

  void _closeWorkspace() => setState(() => workspace = null);

  @override
  void dispose() {
    editor.removeListener(_expressionChanged);
    editor.dispose();
    core.dispose();
    sheetText.dispose();
    toolText.dispose();
    financePrincipal.dispose();
    financeRate.dispose();
    financeMonths.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: workspace == null,
    onPopInvokedWithResult: (didPop, _) {
      if (!didPop && workspace != null) _closeWorkspace();
    },
    child: Scaffold(
      body: SafeArea(
        child: Column(
          children: [
            _header(),
            if (workspace == null) _viewBar(),
            Expanded(child: workspace == null ? _calculator() : _workspace()),
          ],
        ),
      ),
    ),
  );

  Widget _header() => Container(
    height: 58,
    padding: const EdgeInsets.symmetric(horizontal: 12),
    decoration: const BoxDecoration(
      border: Border(bottom: BorderSide(color: border)),
    ),
    child: Row(
      children: [
        Image.asset('assets/icon.png', width: 32, height: 32),
        const SizedBox(width: 8),
        const Expanded(
          child: Text(
            'Scientific Calculator Studio Pro',
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(fontSize: 14, fontWeight: FontWeight.w700),
          ),
        ),
        IconButton(
          onPressed: _showHistory,
          tooltip: 'History',
          icon: const Icon(Icons.history, size: 22),
        ),
        TextButton(
          onPressed: _showTools,
          child: const Text('Tools', style: TextStyle(color: foreground)),
        ),
      ],
    ),
  );

  Widget _viewBar() => Container(
    padding: const EdgeInsets.fromLTRB(12, 7, 12, 4),
    decoration: const BoxDecoration(
      border: Border(bottom: BorderSide(color: border)),
    ),
    child: Column(
      children: [
        Container(
          height: 49,
          padding: const EdgeInsets.all(3),
          decoration: BoxDecoration(
            color: surface,
            border: Border.all(color: border),
            borderRadius: BorderRadius.circular(12),
          ),
          child: Row(
            children: CalcView.values
                .map((item) => Expanded(child: _viewButton(item)))
                .toList(),
          ),
        ),
        SizedBox(
          height: 27,
          child: Row(
            children: [
              Text(switch (view) {
                CalcView.simple => 'Everyday calculations',
                CalcView.semi => 'Quick scientific functions',
                CalcView.pro => 'Advanced math and tools',
              }, style: const TextStyle(color: muted, fontSize: 11)),
              const Spacer(),
              if (view != defaultView)
                TextButton(
                  onPressed: () {
                    setState(() => defaultView = view);
                    unawaited(prefs.setString('defaultView', view.name));
                  },
                  child: const Text(
                    'Open in this view',
                    style: TextStyle(color: orange, fontSize: 11),
                  ),
                ),
            ],
          ),
        ),
      ],
    ),
  );

  Widget _viewButton(CalcView item) {
    final active = view == item;
    return InkWell(
      onTap: () => setState(() {
        view = item;
        mathOpen = false;
      }),
      borderRadius: BorderRadius.circular(9),
      child: Container(
        alignment: Alignment.center,
        decoration: BoxDecoration(
          color: active ? keyColor : Colors.transparent,
          borderRadius: BorderRadius.circular(9),
          border: active ? Border.all(color: const Color(0xFF3A3A40)) : null,
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Flexible(
              child: Text(
                switch (item) {
                  CalcView.simple => 'Simple',
                  CalcView.semi => 'Semi Pro',
                  CalcView.pro => 'Pro',
                },
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w600,
                  color: active ? foreground : muted,
                ),
              ),
            ),
            if (item == defaultView)
              const Text(' •', style: TextStyle(color: orange, fontSize: 14)),
          ],
        ),
      ),
    );
  }

  Widget _calculator() => LayoutBuilder(
    builder: (context, box) {
      final wide = box.maxWidth >= 700;
      final padWidth = wide
          ? math.min(420.0, box.maxWidth * .46)
          : box.maxWidth;
      final quickHeight = view != CalcView.simple && !mathOpen ? 58.0 : 0.0;
      final padHeight = math.min(
        mathOpen ? 350.0 : 400.0,
        wide
            ? math.max(0.0, box.maxHeight - quickHeight)
            : box.maxHeight * (box.maxHeight < 550 ? .52 : .62),
      );
      final pad = SizedBox(
        width: padWidth,
        height: padHeight,
        child: mathOpen ? _mathPad() : _numberPad(),
      );
      final quick = view == CalcView.simple || mathOpen
          ? const SizedBox.shrink()
          : _quickRow();
      if (wide) {
        return Row(
          children: [
            Expanded(
              child: Container(
                decoration: const BoxDecoration(
                  border: Border(right: BorderSide(color: border)),
                ),
                child: _display(wide: true),
              ),
            ),
            SizedBox(
              width: padWidth,
              child: Column(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [quick, pad],
              ),
            ),
          ],
        );
      }
      return Column(
        children: [
          Expanded(child: _display(wide: false)),
          quick,
          pad,
        ],
      );
    },
  );

  Widget _display({required bool wide}) => Padding(
    padding: EdgeInsets.fromLTRB(wide ? 30 : 18, 12, wide ? 30 : 18, 14),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (view != CalcView.simple)
          Align(
            alignment: Alignment.topLeft,
            child: TextButton(
              onPressed: () {
                setState(() => angle = (angle + 1) % 3);
                core.setAngle(angle);
                _expressionChanged();
              },
              child: Text(
                ['DEG', 'RAD', 'GRAD'][angle],
                style: const TextStyle(color: orange, fontSize: 12),
              ),
            ),
          ),
        Expanded(
          child: LayoutBuilder(
            builder: (context, bounds) => SingleChildScrollView(
              reverse: true,
              child: ConstrainedBox(
                constraints: BoxConstraints(minHeight: bounds.maxHeight),
                child: Column(
                  mainAxisAlignment:
                      !mathOpen && history.isNotEmpty && bounds.maxHeight >= 300
                      ? MainAxisAlignment.spaceBetween
                      : MainAxisAlignment.end,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (!mathOpen &&
                        history.isNotEmpty &&
                        bounds.maxHeight >= 300)
                      _recentCalculations(bounds.maxHeight >= 450 ? 3 : 2),
                    Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        if (view == CalcView.pro &&
                            MediaQuery.sizeOf(context).height >= 720)
                          Text(
                            mathOpen
                                ? 'Tap a slot to edit · Numbers returns to the keypad'
                                : 'Tap Math for structured keys',
                            style: const TextStyle(color: muted, fontSize: 12),
                          ),
                        if (mathOpen || tex.isNotEmpty)
                          const SizedBox(height: 12),
                        if (mathOpen || tex.isNotEmpty)
                          Row(
                            children: [
                              Expanded(
                                child: SingleChildScrollView(
                                  scrollDirection: Axis.horizontal,
                                  reverse: true,
                                  child: mathOpen
                                      ? SizedBox(
                                          width: math.max(
                                            260,
                                            MediaQuery.sizeOf(context).width *
                                                (wide ? .42 : .70),
                                          ),
                                          child: MathField(
                                            controller: editor,
                                            opensKeyboard: false,
                                            variables: const [
                                              'x',
                                              'y',
                                              'R',
                                              'C',
                                              'k',
                                              'n',
                                            ],
                                            decoration: const InputDecoration(
                                              border: InputBorder.none,
                                              isDense: true,
                                            ),
                                          ),
                                        )
                                      : fm.Math.tex(
                                          tex,
                                          textStyle: const TextStyle(
                                            fontSize: 25,
                                            color: foreground,
                                          ),
                                          onErrorFallback: (_) => Text(
                                            tex,
                                            style: const TextStyle(
                                              color: muted,
                                              fontSize: 20,
                                            ),
                                          ),
                                        ),
                                ),
                              ),
                              if (!mathOpen)
                                IconButton(
                                  onPressed: () =>
                                      editor.goBack(deleteMode: true),
                                  icon: const Icon(Icons.backspace_outlined),
                                  color: muted,
                                  tooltip: 'Delete last symbol',
                                ),
                            ],
                          ),
                        if (mathOpen || tex.isNotEmpty)
                          const SizedBox(height: 12),
                        Text(
                          error ?? result,
                          textAlign: TextAlign.right,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            color: error == null ? foreground : orange,
                            fontSize: view == CalcView.simple ? 56 : 36,
                            fontWeight: FontWeight.w500,
                            letterSpacing: -1.5,
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ],
    ),
  );

  Widget _recentCalculations(int limit) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      const Text(
        'RECENT · TAP TO REUSE',
        style: TextStyle(
          color: muted,
          fontSize: 10,
          fontWeight: FontWeight.w700,
          letterSpacing: 1,
        ),
      ),
      const SizedBox(height: 8),
      for (final item in history.take(limit))
        InkWell(
          onTap: () => _restoreHistory(item),
          borderRadius: BorderRadius.circular(10),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 9),
            child: Row(
              children: [
                Expanded(
                  child: SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: fm.Math.tex(
                      item.tex,
                      textStyle: const TextStyle(color: muted, fontSize: 16),
                      onErrorFallback: (_) => Text(
                        item.tex,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(color: muted, fontSize: 14),
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 10),
                ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 110),
                  child: Text(
                    item.result,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(color: foreground, fontSize: 16),
                  ),
                ),
              ],
            ),
          ),
        ),
    ],
  );

  Widget _quickRow() => Container(
    height: 58,
    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
    decoration: const BoxDecoration(
      color: surface,
      border: Border(top: BorderSide(color: border)),
    ),
    child: Row(
      children: [
        for (final name in shortcuts)
          Expanded(
            child: Padding(
              padding: const EdgeInsets.only(right: 5),
              child: _key(name, () => _shortcut(name), small: true),
            ),
          ),
        Expanded(
          child: _key(
            'Math ▴',
            () => setState(() {
              mathOpen = true;
              mathPage = MathPage.structures;
            }),
            accent: true,
            small: true,
          ),
        ),
      ],
    ),
  );

  Widget _numberPad() {
    const rows = [
      ['AC', '±', '%', '÷'],
      ['7', '8', '9', '×'],
      ['4', '5', '6', '−'],
      ['1', '2', '3', '+'],
      ['0', '.', '='],
    ];
    return Container(
      color: surface,
      padding: const EdgeInsets.fromLTRB(10, 7, 10, 12),
      child: Column(
        children: [
          for (final row in rows)
            Expanded(
              child: Padding(
                padding: const EdgeInsets.only(bottom: 5),
                child: Row(
                  children: [
                    for (final label in row)
                      Expanded(
                        flex: label == '0' ? 2 : 1,
                        child: Padding(
                          padding: const EdgeInsets.only(right: 5),
                          child: _key(label, () => _numberKey(label)),
                        ),
                      ),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }

  Widget _mathPad() {
    final labels = switch (mathPage) {
      MathPage.functions => [
        'sin',
        'cos',
        'tan',
        'ln',
        'log',
        '√',
        'x²',
        'x⁻¹',
        'eˣ',
        '|□|',
        'n!',
        'mod',
      ],
      MathPage.symbols => [
        'α',
        'β',
        'θ',
        'Ω',
        'x',
        'R',
        'C',
        'µ',
        'π',
        '∞',
        'Ans',
        '∠',
      ],
      MathPage.structures => [
        '□/□',
        'x□',
        '√□',
        'ⁿ√□',
        '(□)',
        'x²',
        'x⁻¹',
        '|x|',
        '(',
        ')',
        ',',
        'n!',
      ],
    };
    return Container(
      color: surface,
      child: Column(
        children: [
          SizedBox(
            height: 48,
            child: Row(
              children: [
                for (final label in [
                  'Numbers',
                  'Functions',
                  'Symbols',
                  'Structures',
                ])
                  Expanded(
                    child: InkWell(
                      onTap: () => label == 'Numbers'
                          ? setState(() => mathOpen = false)
                          : setState(
                              () => mathPage = MathPage.values.firstWhere(
                                (item) => item.name == label.toLowerCase(),
                              ),
                            ),
                      child: Container(
                        alignment: Alignment.center,
                        decoration: BoxDecoration(
                          border: Border(
                            bottom: BorderSide(
                              color: label.toLowerCase() == mathPage.name
                                  ? orange
                                  : border,
                              width: label.toLowerCase() == mathPage.name
                                  ? 2
                                  : 1,
                            ),
                          ),
                        ),
                        child: Text(
                          label,
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w600,
                            color: label.toLowerCase() == mathPage.name
                                ? orange
                                : muted,
                          ),
                        ),
                      ),
                    ),
                  ),
              ],
            ),
          ),
          Expanded(
            child: Column(
              children: [
                for (var row = 0; row < 3; row++)
                  Expanded(
                    child: Row(
                      children: [
                        for (var col = 0; col < 4; col++)
                          Expanded(
                            child: Padding(
                              padding: const EdgeInsets.all(3),
                              child: _key(
                                labels[row * 4 + col],
                                () => _mathKey(labels[row * 4 + col]),
                                small: true,
                              ),
                            ),
                          ),
                      ],
                    ),
                  ),
              ],
            ),
          ),
          SizedBox(
            height: 51,
            child: Row(
              children: [
                Expanded(child: _key('←', () => editor.goBack(), small: true)),
                Expanded(child: _key('→', () => editor.goNext(), small: true)),
                Expanded(
                  flex: 2,
                  child: _key(
                    'Next slot ⇥',
                    () => editor.goNext(),
                    small: true,
                    accent: true,
                  ),
                ),
                Expanded(
                  child: _key(
                    '⌫',
                    () => editor.goBack(deleteMode: true),
                    small: true,
                  ),
                ),
                Expanded(child: _key('=', _commit, small: true)),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _key(
    String label,
    VoidCallback onTap, {
    bool small = false,
    bool accent = false,
  }) {
    final operator = ['÷', '×', '−', '+'].contains(label);
    final utility = ['AC', '±', '%'].contains(label);
    final equal = label == '=';
    return Material(
      color: equal
          ? orange
          : accent
          ? const Color(0xFF38251F)
          : keyColor,
      borderRadius: BorderRadius.circular(10),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(10),
        child: Container(
          alignment: Alignment.center,
          decoration: BoxDecoration(
            border: Border.all(
              color: equal
                  ? orange
                  : accent
                  ? const Color(0xFF68402F)
                  : border,
            ),
            borderRadius: BorderRadius.circular(10),
          ),
          child: Text(
            label,
            textAlign: TextAlign.center,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(
              fontSize: small ? 14 : 24,
              fontWeight: equal ? FontWeight.w700 : FontWeight.w500,
              color: equal
                  ? const Color(0xFF16110F)
                  : operator || accent
                  ? orange
                  : utility
                  ? muted
                  : foreground,
            ),
          ),
        ),
      ),
    );
  }

  Widget _workspace() {
    final name = workspace!;
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(18, 14, 12, 10),
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'FOCUSED WORKSPACE',
                      style: TextStyle(
                        color: orange,
                        fontSize: 10,
                        fontWeight: FontWeight.w700,
                        letterSpacing: 1.7,
                      ),
                    ),
                    Text(
                      name,
                      style: const TextStyle(
                        fontSize: 27,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ],
                ),
              ),
              TextButton.icon(
                onPressed: _closeWorkspace,
                icon: const Icon(Icons.arrow_back, size: 18),
                label: const Text('Calculator'),
              ),
            ],
          ),
        ),
        _sourceCard(),
        Expanded(
          child: name == 'Graph'
              ? _graphWorkspace()
              : name == 'Sheet'
              ? _sheetWorkspace()
              : name == 'Finance'
              ? _financeWorkspace()
              : _expressionWorkspace(name),
        ),
      ],
    );
  }

  Widget _sourceCard() => Container(
    width: double.infinity,
    margin: const EdgeInsets.fromLTRB(14, 0, 14, 12),
    padding: const EdgeInsets.all(14),
    decoration: BoxDecoration(
      color: surface,
      border: Border.all(color: border),
      borderRadius: BorderRadius.circular(12),
    ),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text(
          'CURRENT CALCULATOR EXPRESSION',
          style: TextStyle(
            color: muted,
            fontSize: 10,
            fontWeight: FontWeight.w700,
            letterSpacing: 1,
          ),
        ),
        const SizedBox(height: 6),
        if (tex.isEmpty)
          const Text('No expression yet', style: TextStyle(fontSize: 16))
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: fm.Math.tex(
              tex,
              textStyle: const TextStyle(fontSize: 16),
              onErrorFallback: (_) =>
                  Text(tex, style: const TextStyle(fontSize: 16)),
            ),
          ),
        if (tex.isNotEmpty)
          Text(result, style: const TextStyle(color: orange, fontSize: 13)),
      ],
    ),
  );

  Widget _graphWorkspace() {
    final expression = engineExpression.isEmpty ? 'x^2' : engineExpression;
    return LayoutBuilder(
      builder: (context, constraints) => SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(14, 0, 14, 24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'y = $expression',
              style: const TextStyle(fontSize: 17, color: foreground),
            ),
            const SizedBox(height: 10),
            AspectRatio(
              aspectRatio: constraints.maxWidth > 700 ? 2.2 : 1.3,
              child: Container(
                clipBehavior: Clip.hardEdge,
                decoration: BoxDecoration(
                  color: surface,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: border),
                ),
                child: GestureDetector(
                  onScaleStart: (_) => _graphStartRange = graphRange,
                  onScaleUpdate: (details) => setState(() {
                    graphRange = (_graphStartRange / details.scale).clamp(
                      .2,
                      100000,
                    );
                    graphCenter -=
                        details.focalPointDelta.dx /
                        math.max(1, constraints.maxWidth) *
                        graphRange *
                        2;
                  }),
                  child: CustomPaint(
                    painter: _GraphPainter(
                      expression: expression,
                      core: core,
                      range: graphRange,
                      center: graphCenter,
                    ),
                  ),
                ),
              ),
            ),
            const SizedBox(height: 10),
            const Text(
              'Pinch to zoom · drag to inspect',
              style: TextStyle(color: muted, fontSize: 12),
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              children: [
                OutlinedButton(
                  onPressed: () => setState(() {
                    graphRange /= 1.5;
                  }),
                  child: const Text('Zoom in'),
                ),
                OutlinedButton(
                  onPressed: () => setState(() {
                    graphRange *= 1.5;
                  }),
                  child: const Text('Zoom out'),
                ),
                OutlinedButton(
                  onPressed: () => setState(() {
                    graphRange = 10;
                    graphCenter = 0;
                  }),
                  child: const Text('Reset view'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  double _graphStartRange = 10;

  Widget _sheetWorkspace() => SingleChildScrollView(
    padding: const EdgeInsets.fromLTRB(14, 0, 14, 24),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const Text(
          'One expression or assignment per line. Values carry into later lines.',
          style: TextStyle(color: muted, fontSize: 13),
        ),
        Align(
          alignment: Alignment.centerLeft,
          child: TextButton.icon(
            onPressed: () => setState(() {
              sheetText.text = sheetExample;
              sheetLines = [];
            }),
            icon: const Icon(Icons.science_outlined, size: 18),
            label: const Text('Load example'),
          ),
        ),
        const SizedBox(height: 10),
        TextField(
          controller: sheetText,
          minLines: 5,
          maxLines: 12,
          keyboardType: TextInputType.multiline,
          decoration: _fieldDecoration('Enter expressions'),
        ),
        const SizedBox(height: 10),
        FilledButton.icon(
          onPressed: () {
            setState(() {
              try {
                sheetLines = core.runSheet(sheetText.text);
              } on FormatException catch (e) {
                sheetLines = [SheetLine('error', e.message)];
              }
            });
            unawaited(prefs.setString('sheet', sheetText.text));
          },
          icon: const Icon(Icons.play_arrow),
          label: const Text('Calculate sheet'),
        ),
        const SizedBox(height: 12),
        for (var i = 0; i < sheetLines.length; i++)
          Container(
            margin: const EdgeInsets.only(bottom: 5),
            padding: const EdgeInsets.all(11),
            decoration: BoxDecoration(
              color: surface,
              border: Border.all(color: border),
              borderRadius: BorderRadius.circular(9),
            ),
            child: Row(
              children: [
                SizedBox(
                  width: 24,
                  child: Text('${i + 1}', style: const TextStyle(color: muted)),
                ),
                Expanded(
                  child: Text(
                    sheetLines[i].text.isEmpty ? '—' : sheetLines[i].text,
                    style: TextStyle(
                      color: sheetLines[i].kind == 'error'
                          ? orange
                          : foreground,
                    ),
                  ),
                ),
              ],
            ),
          ),
      ],
    ),
  );

  Widget _expressionWorkspace(String name) => SingleChildScrollView(
    padding: const EdgeInsets.fromLTRB(14, 0, 14, 24),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(switch (name) {
          'Matrix' => 'Determinant, inverse, rank, eigenvalues and more',
          'Equation' => 'Solve, differentiate and integrate expressions',
          'Statistics' => 'Summarize a data set or use distributions',
          'Convert' => 'Convert values between supported units',
          'Programmer' => 'Bitwise and base calculations',
          _ => 'Enter an expression',
        }, style: const TextStyle(color: muted, fontSize: 13)),
        const SizedBox(height: 10),
        TextField(
          controller: toolText,
          minLines: 1,
          maxLines: 4,
          decoration: _fieldDecoration('Enter an expression'),
          onSubmitted: (_) => _runTool(),
        ),
        const SizedBox(height: 10),
        FilledButton.icon(
          onPressed: _runTool,
          icon: const Icon(Icons.calculate),
          label: const Text('Calculate'),
        ),
        if (toolResult != null)
          Container(
            margin: const EdgeInsets.only(top: 14),
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: surface,
              border: Border.all(color: border),
              borderRadius: BorderRadius.circular(12),
            ),
            child: SelectableText(
              toolResult!,
              style: const TextStyle(fontSize: 24, color: foreground),
            ),
          ),
        const SizedBox(height: 16),
        Text(
          'Try another ${name.toLowerCase()} expression or return to the calculator.',
          style: const TextStyle(color: muted, fontSize: 12),
        ),
      ],
    ),
  );

  void _runTool() {
    final reply = core.calculate(toolText.text.trim(), commit: true);
    setState(() => toolResult = reply.text);
  }

  Widget _financeWorkspace() => SingleChildScrollView(
    padding: const EdgeInsets.fromLTRB(14, 0, 14, 24),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const Text(
          'Monthly loan payment',
          style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: 10),
        _financeField(financePrincipal, 'Principal'),
        _financeField(financeRate, 'Annual interest rate (%)'),
        _financeField(financeMonths, 'Number of months'),
        FilledButton.icon(
          onPressed: () {
            final principal = double.tryParse(financePrincipal.text);
            final rate = double.tryParse(financeRate.text);
            final months = double.tryParse(financeMonths.text);
            if (principal == null ||
                rate == null ||
                months == null ||
                principal <= 0 ||
                rate < 0 ||
                months <= 0) {
              setState(
                () => toolResult =
                    'Enter a positive principal and term, and a non-negative rate.',
              );
              return;
            }
            setState(
              () => toolResult = core
                  .loanPayment(principal, rate, months)
                  .toStringAsFixed(2),
            );
          },
          icon: const Icon(Icons.calculate),
          label: const Text('Calculate payment'),
        ),
        if (toolResult != null)
          Padding(
            padding: const EdgeInsets.only(top: 18),
            child: Text(
              toolResult!,
              style: const TextStyle(fontSize: 30, color: orange),
            ),
          ),
      ],
    ),
  );

  Widget _financeField(TextEditingController controller, String label) =>
      Padding(
        padding: const EdgeInsets.only(bottom: 9),
        child: TextField(
          controller: controller,
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
          decoration: _fieldDecoration(label),
        ),
      );

  InputDecoration _fieldDecoration(String label) => InputDecoration(
    labelText: label,
    filled: true,
    fillColor: surface,
    border: OutlineInputBorder(
      borderSide: const BorderSide(color: border),
      borderRadius: BorderRadius.circular(10),
    ),
  );

  void _showTools() {
    var query = '';
    var showSettings = false;
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      backgroundColor: surface,
      useSafeArea: true,
      builder: (sheetContext) => StatefulBuilder(
        builder: (context, update) {
          final tools = toolNames
              .where((name) => name.toLowerCase().contains(query))
              .toList();
          final functions = shortcutNames
              .where((name) => name.toLowerCase().contains(query))
              .toList();
          final advanced = advancedFunctionExamples.keys
              .where((name) => name.toLowerCase().contains(query))
              .toList();
          return FractionallySizedBox(
            heightFactor: .88,
            child: Padding(
              padding: EdgeInsets.fromLTRB(
                16,
                12,
                16,
                MediaQuery.viewInsetsOf(context).bottom + 12,
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      const Expanded(
                        child: Text(
                          'Tools',
                          style: TextStyle(
                            fontSize: 25,
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                      ),
                      TextButton.icon(
                        onPressed: () {
                          FocusScope.of(context).unfocus();
                          update(() {
                            showSettings = !showSettings;
                            query = '';
                          });
                        },
                        icon: Icon(
                          showSettings ? Icons.apps : Icons.tune,
                          size: 18,
                        ),
                        label: Text(showSettings ? 'Browse' : 'Shortcuts'),
                      ),
                      IconButton(
                        onPressed: () => Navigator.pop(sheetContext),
                        icon: const Icon(Icons.close),
                        tooltip: 'Close tools',
                      ),
                    ],
                  ),
                  if (!showSettings)
                    TextField(
                      onChanged: (value) =>
                          update(() => query = value.trim().toLowerCase()),
                      decoration: _fieldDecoration(
                        'Search tools and functions',
                      ).copyWith(prefixIcon: const Icon(Icons.search)),
                    ),
                  if (!showSettings) const SizedBox(height: 10),
                  Expanded(
                    child: ListView(
                      children: [
                        if (!showSettings) ...[
                          if (tools.isNotEmpty)
                            const _SectionLabel('WORKSPACES'),
                          for (final name in tools)
                            ListTile(
                              leading: Icon(_toolIcon(name), color: orange),
                              title: Text(name),
                              trailing: const Icon(
                                Icons.chevron_right,
                                color: muted,
                              ),
                              onTap: () {
                                Navigator.pop(sheetContext);
                                _openWorkspace(name);
                              },
                            ),
                          if (functions.isNotEmpty)
                            const _SectionLabel('QUICK FUNCTIONS'),
                          for (final name in functions)
                            ListTile(
                              title: Text(name),
                              subtitle: const Text(
                                'Insert into current expression',
                              ),
                              onTap: () {
                                Navigator.pop(sheetContext);
                                _shortcut(name);
                              },
                            ),
                          if (advanced.isNotEmpty)
                            const _SectionLabel('ADVANCED FUNCTIONS'),
                          for (final name in advanced)
                            ListTile(
                              title: Text(name),
                              subtitle: Text(
                                'Open ${advancedFunctionExamples[name]![0]}',
                              ),
                              trailing: const Icon(
                                Icons.chevron_right,
                                color: muted,
                              ),
                              onTap: () {
                                Navigator.pop(sheetContext);
                                final target = advancedFunctionExamples[name]!;
                                _openWorkspace(target[0], example: target[1]);
                              },
                            ),
                        ],
                        if (showSettings) ...[
                          const SizedBox(height: 12),
                          const _SectionLabel(
                            'ADAPTIVE SHORTCUTS · ON THIS DEVICE',
                          ),
                          SwitchListTile(
                            contentPadding: EdgeInsets.zero,
                            title: const Text('Suggest frequent functions'),
                            value: adaptive,
                            onChanged: (value) {
                              setState(() => adaptive = value);
                              update(() {});
                              _saveShortcuts();
                            },
                          ),
                          for (final name in shortcutNames)
                            Row(
                              children: [
                                Expanded(
                                  child: Text(
                                    name,
                                    style: const TextStyle(fontSize: 15),
                                  ),
                                ),
                                TextButton(
                                  onPressed: () {
                                    setState(() {
                                      if (!pinned.remove(name)) {
                                        pinned.add(name);
                                        hidden.remove(name);
                                      }
                                    });
                                    update(() {});
                                    _saveShortcuts();
                                  },
                                  child: Text(
                                    pinned.contains(name) ? 'Unpin' : 'Pin',
                                  ),
                                ),
                                TextButton(
                                  onPressed: () {
                                    setState(() {
                                      if (!hidden.remove(name)) {
                                        hidden.add(name);
                                        pinned.remove(name);
                                      }
                                    });
                                    update(() {});
                                    _saveShortcuts();
                                  },
                                  child: Text(
                                    hidden.contains(name) ? 'Show' : 'Hide',
                                  ),
                                ),
                              ],
                            ),
                          TextButton.icon(
                            onPressed: () {
                              setState(() {
                                usage.clear();
                                pinned.clear();
                                hidden.clear();
                                adaptive = true;
                              });
                              update(() {});
                              _saveShortcuts();
                            },
                            icon: const Icon(Icons.restart_alt),
                            label: const Text('Reset suggestions'),
                          ),
                        ],
                        if (!showSettings &&
                            tools.isEmpty &&
                            functions.isEmpty &&
                            advanced.isEmpty)
                          const Padding(
                            padding: EdgeInsets.all(24),
                            child: Text('No matching tools'),
                          ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }

  IconData _toolIcon(String name) => switch (name) {
    'Graph' => Icons.show_chart,
    'Sheet' => Icons.table_chart_outlined,
    'Matrix' => Icons.grid_on,
    'Equation' => Icons.functions,
    'Statistics' => Icons.bar_chart,
    'Finance' => Icons.account_balance,
    'Convert' => Icons.swap_horiz,
    _ => Icons.memory,
  };

  void _showHistory() => showModalBottomSheet<void>(
    context: context,
    backgroundColor: surface,
    isScrollControlled: true,
    useSafeArea: true,
    builder: (sheetContext) => FractionallySizedBox(
      heightFactor: .7,
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                const Expanded(
                  child: Text(
                    'History',
                    style: TextStyle(fontSize: 24, fontWeight: FontWeight.w700),
                  ),
                ),
                IconButton(
                  onPressed: () => Navigator.pop(sheetContext),
                  icon: const Icon(Icons.close),
                ),
              ],
            ),
          ),
          Expanded(
            child: history.isEmpty
                ? const Center(
                    child: Text(
                      'No calculations yet',
                      style: TextStyle(color: muted),
                    ),
                  )
                : ListView.builder(
                    itemCount: history.length,
                    itemBuilder: (context, index) => ListTile(
                      title: Text(
                        history[index].tex,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      subtitle: Text(
                        history[index].result,
                        style: const TextStyle(color: orange),
                      ),
                      onTap: () {
                        Navigator.pop(sheetContext);
                        _restoreHistory(history[index]);
                      },
                    ),
                  ),
          ),
        ],
      ),
    ),
  );

  void _restoreHistory(HistoryItem item) {
    editor.clear();
    for (final rune in item.tex.runes) {
      editor.addLeaf(String.fromCharCode(rune));
    }
    setState(() {
      result = item.result;
      error = null;
    });
  }
}

class _SectionLabel extends StatelessWidget {
  const _SectionLabel(this.label);
  final String label;
  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(0, 8, 0, 5),
    child: Text(
      label,
      style: const TextStyle(
        color: muted,
        fontSize: 11,
        fontWeight: FontWeight.w700,
        letterSpacing: 1.4,
      ),
    ),
  );
}

class _GraphPainter extends CustomPainter {
  const _GraphPainter({
    required this.expression,
    required this.core,
    required this.range,
    required this.center,
  });
  final String expression;
  final CalculatorCore core;
  final double range, center;

  @override
  void paint(Canvas canvas, Size size) {
    final samples = <(double, double)>[];
    for (var p = 0; p <= size.width.ceil(); p += 2) {
      final x = center + (p / size.width * 2 - 1) * range;
      samples.add((p.toDouble(), core.at(expression, x)));
    }
    final finite =
        samples
            .map((sample) => sample.$2)
            .where((value) => value.isFinite)
            .toList()
          ..sort();
    if (finite.isEmpty) {
      final label = TextPainter(
        text: const TextSpan(
          text: 'This expression cannot be graphed',
          style: TextStyle(color: muted, fontSize: 14),
        ),
        textDirection: TextDirection.ltr,
      )..layout(maxWidth: size.width - 24);
      label.paint(canvas, Offset(12, size.height / 2 - label.height / 2));
      return;
    }
    final low = finite[(finite.length * .05).floor()];
    final high = finite[(finite.length * .95).floor()];
    final yCenter = (low + high) / 2;
    final ySpan = math.max(2.0, (high - low) * 1.3);
    final yMin = yCenter - ySpan / 2;
    final yMax = yCenter + ySpan / 2;
    final grid = Paint()
      ..color = const Color(0xFF35353B)
      ..strokeWidth = 1;
    final axis = Paint()
      ..color = muted
      ..strokeWidth = 1.3;
    final curve = Paint()
      ..color = orange
      ..strokeWidth = 2.8
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;
    for (var i = 1; i < 5; i++) {
      final x = size.width * i / 5;
      final y = size.height * i / 5;
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), grid);
      canvas.drawLine(Offset(0, y), Offset(size.width, y), grid);
    }
    final originX = (0.5 - center / (2 * range)) * size.width;
    if (originX >= 0 && originX <= size.width) {
      canvas.drawLine(Offset(originX, 0), Offset(originX, size.height), axis);
    }
    if (yMin <= 0 && yMax >= 0) {
      final zeroY = (yMax / ySpan) * size.height;
      canvas.drawLine(Offset(0, zeroY), Offset(size.width, zeroY), axis);
    }
    var path = Path();
    var active = false;
    for (final (p, y) in samples) {
      final pointY = (yMax - y) / ySpan * size.height;
      if (!y.isFinite || pointY < -20 || pointY > size.height + 20) {
        if (active) canvas.drawPath(path, curve);
        active = false;
        continue;
      }
      if (!active) {
        path = Path()..moveTo(p.toDouble(), pointY);
        active = true;
      } else {
        path.lineTo(p.toDouble(), pointY);
      }
    }
    if (active) canvas.drawPath(path, curve);
  }

  @override
  bool shouldRepaint(covariant _GraphPainter old) =>
      old.expression != expression ||
      old.range != range ||
      old.center != center;
}
