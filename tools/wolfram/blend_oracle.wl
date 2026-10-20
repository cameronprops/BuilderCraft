(* Generate independent analytic references for the shared Worldwright blend solver.
   Run: wolframscript -file tools/wolfram/blend_oracle.wl > crates/kernel/tests/fixtures/wolfram_blend_oracle.json
   Inputs and expected results use exact Wolfram rationals until final JSON export.
   Runtime Worldwright never requires Wolfram. *)
ClearAll[weightedBezier, pointCurvature, makeCase, oracle];
weightedBezier[points_, weights_, t_] := Module[{n = Length[points] - 1, b},
 b = Table[Binomial[n, i] (1 - t)^(n - i) t^i, {i, 0, n}];
 Total[MapThread[#1 #2 #3 &, {b, weights, points}]]/(b.weights)
];
pointCurvature[expression_, variable_] := Module[{v, a},
 v = D[expression, variable]; a = D[expression, {variable, 2}];
 Together[(a - v (a.v)/(v.v))/(v.v)]
];
makeCase[name_, ap_, aw_, bp_, bw_, level_, sa_, sb_, ra_, rb_] := Module[
 {u, ac, bc, at, bt, start, finish, va, vb, ka, kb, h1, h2, cp, expr, steps, k},
 ac = weightedBezier[ap, aw, u]; bc = weightedBezier[bp, bw, u];
 at = If[ra == 0, 1, 0]; bt = If[rb == 0, 0, 1];
 start = Simplify[ac /. u -> at]; finish = Simplify[bc /. u -> bt];
 va = Simplify[D[ac, u] /. u -> at] If[ra == 0, 1, -1];
 vb = Simplify[D[bc, u] /. u -> bt] If[rb == 0, 1, -1];
 cp = If[level == 0, {start, finish},
  h1 = Simplify[va EuclideanDistance[start, finish] sa/Sqrt[va.va]];
  h2 = Simplify[vb EuclideanDistance[start, finish] sb/Sqrt[vb.vb]];
  If[level == 1, {start, start + h1, finish - h2, finish},
   ka = Simplify[pointCurvature[ac, u] /. u -> at];
   kb = Simplify[pointCurvature[bc, u] /. u -> bt];
   {start, start + h1, Simplify[start + 2 h1 + (5 h1. (5 h1)) ka/20],
    Simplify[finish - 2 h2 + (5 h2. (5 h2)) kb/20], finish - h2, finish}
  ]
 ];
 expr = weightedBezier[cp, ConstantArray[1, Length[cp]], u];
 steps = Table[<|"t" -> N[s, 16], "point" -> N[expr /. u -> s, 16],
    "derivative" -> N[D[expr, u] /. u -> s, 16]|>, {s, {0, 1/4, 1/2, 3/4, 1}}];
 k = If[level == 2, <|"start" -> N[pointCurvature[expr, u] /. u -> 0, 16],
     "mid" -> N[pointCurvature[expr, u] /. u -> 1/2, 16],
     "end" -> N[pointCurvature[expr, u] /. u -> 1, 16]|>, Null];
 <|"name" -> name,
   "a" -> <|"degree" -> Length[ap] - 1, "control" -> N[ap, 16], "weights" -> N[aw, 16]|>,
   "b" -> <|"degree" -> Length[bp] - 1, "control" -> N[bp, 16], "weights" -> N[bw, 16]|>,
   "continuity" -> level, "strength_a" -> N[sa, 16], "strength_b" -> N[sb, 16],
   "reverse_a" -> ra, "reverse_b" -> rb,
   "expected" -> <|"degree" -> Length[cp] - 1, "control" -> N[cp, 16],
      "stations" -> steps, "curvature" -> k|>|>
];
oracle[] := ExportString[<|
 "schema_version" -> 1, "generator" -> "Wolfram Language exact rational Bezier algebra, 2026-10-20",
 "cases" -> {
 makeCase["g0_xyz", {{0,0,0},{2,0,0}}, {1,1}, {{4,2,1},{4,4,1}}, {1,1}, 0, 1/3, 1/3, 0, 0],
 makeCase["g1_xyz", {{0,0,0},{2,0,0}}, {1,1}, {{4,2,1},{4,4,1}}, {1,1}, 1, 1/4, 1/2, 0, 0],
 makeCase["g2_quadratic", {{0,0,0},{1,0,0},{2,1,0}}, {1,1,1},
           {{4,2,0},{5,3,0},{6,3,0}}, {1,1,1}, 2, 7/20, 9/20, 0, 0],
 makeCase["g2_rational_reversed", {{1,0,0},{1,1,0},{0,1,0}}, {1,1/Sqrt[2],1},
           {{-3/2,3,0},{-2,4,0},{-1,5,0}}, {1,1,1}, 2, 1/4, 1/4, 0, 1]
 }
|>, "RawJSON", "Compact" -> True];

Print[oracle[]];
