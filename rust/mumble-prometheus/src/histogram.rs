// Copyright 2022 The Prometheus Authors
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use libm::ldexp;

pub fn get_bound(idx: i32, schema: i32) -> f64 {
    // Here a bit of context about the behavior for the last bucket counting
    // regular numbers (called simply "last bucket" below) and the bucket
    // counting observations of ±Inf (called "inf bucket" below, with an idx
    // one higher than that of the "last bucket"):
    //
    // If we apply the usual formula to the last bucket, its upper bound
    // would be calculated as +Inf. The reason is that the max possible
    // regular float64 number (math.MaxFloat64) doesn't coincide with one of
    // the calculated bucket boundaries. So the calculated boundary has to
    // be larger than math.MaxFloat64, and the only float64 larger than
    // math.MaxFloat64 is +Inf. However, we want to count actual
    // observations of ±Inf in the inf bucket. Therefore, we have to treat
    // the upper bound of the last bucket specially and set it to
    // math.MaxFloat64. (The upper bound of the inf bucket, with its idx
    // being one higher than that of the last bucket, naturally comes out as
    // +Inf by the usual formula. So that's fine.)
    //
    // math.MaxFloat64 has a frac of 0.9999999999999999 and an exp of
    // 1024. If there were a float64 number following math.MaxFloat64, it
    // would have a frac of 1.0 and an exp of 1024, or equivalently a frac
    // of 0.5 and an exp of 1025. However, since frac must be smaller than
    // 1, and exp must be smaller than 1025, either representation overflows
    // a float64. (Which, in turn, is the reason that math.MaxFloat64 is the
    // largest possible float64. Q.E.D.) However, the formula for
    // calculating the upper bound from the idx and schema of the last
    // bucket results in precisely that. It is either frac=1.0 & exp=1024
    // (for schema < 0) or frac=0.5 & exp=1025 (for schema >=0). (This is,
    // by the way, a power of two where the exponent itself is a power of
    // two, 2¹⁰ in fact, which coinicides with a bucket boundary in all
    // schemas.) So these are the special cases we have to catch below.
    if schema < 0 {
        let exp = idx << -schema;
        if exp == 1024 {
            // This is the last bucket before the overflow bucket
            // (for ±Inf observations). Return math.MaxFloat64 as
            // explained above.
            return f64::MAX;
        }
        return ldexp(1.0, exp);
    }

    let frac_idx = idx & ((1 << schema) - 1);
    let frac = EXPONENTIAL_BOUNDS[schema as usize][frac_idx as usize];
    let exp = (idx >> schema) + 1;
    if frac == 0.5 && exp == 1025 {
        // This is the last bucket before the overflow bucket (for ±Inf
        // observations). Return math.MaxFloat64 as explained above.
        return f64::MAX;
    }
    ldexp(frac, exp)
}

/// EXPONENTIAL_BOUNDS is a precalculated table of bucket bounds in the interval
/// [0.5,1) in schema 0 to 8.
pub const EXPONENTIAL_BOUNDS: &[&[f64]] = &[
    // Schema "0":
    &[0.5],
    // Schema 1:
    &[0.5, 0.7071067811865475],
    // Schema 2:
    &[
        0.5,
        0.5946035575013605,
        0.7071067811865475,
        0.8408964152537144,
    ],
    // Schema 3:
    &[
        0.5,
        0.5452538663326288,
        0.5946035575013605,
        0.6484197773255048,
        0.7071067811865475,
        0.7711054127039704,
        0.8408964152537144,
        0.9170040432046711,
    ],
    // Schema 4:
    &[
        0.5,
        0.5221368912137069,
        0.5452538663326288,
        0.5693943173783458,
        0.5946035575013605,
        0.620928906036742,
        0.6484197773255048,
        0.6771277734684463,
        0.7071067811865475,
        0.7384130729697496,
        0.7711054127039704,
        0.805245165974627,
        0.8408964152537144,
        0.8781260801866495,
        0.9170040432046711,
        0.9576032806985735,
    ],
    // Schema 5:
    &[
        0.5,
        0.5109485743270583,
        0.5221368912137069,
        0.5335702003384117,
        0.5452538663326288,
        0.5571933712979462,
        0.5693943173783458,
        0.5818624293887887,
        0.5946035575013605,
        0.6076236799902344,
        0.620928906036742,
        0.6345254785958666,
        0.6484197773255048,
        0.6626183215798706,
        0.6771277734684463,
        0.6919549409819159,
        0.7071067811865475,
        0.7225904034885232,
        0.7384130729697496,
        0.7545822137967112,
        0.7711054127039704,
        0.7879904225539431,
        0.805245165974627,
        0.8228777390769823,
        0.8408964152537144,
        0.8593096490612387,
        0.8781260801866495,
        0.8973545375015533,
        0.9170040432046711,
        0.9370838170551498,
        0.9576032806985735,
        0.9785720620876999,
    ],
    // Schema 6:
    &[
        0.5,
        0.5054446430258502,
        0.5109485743270583,
        0.5165124395106142,
        0.5221368912137069,
        0.5278225891802786,
        0.5335702003384117,
        0.5393803988785598,
        0.5452538663326288,
        0.5511912916539204,
        0.5571933712979462,
        0.5632608093041209,
        0.5693943173783458,
        0.5755946149764913,
        0.5818624293887887,
        0.5881984958251406,
        0.5946035575013605,
        0.6010783657263515,
        0.6076236799902344,
        0.6142402680534349,
        0.620928906036742,
        0.6276903785123455,
        0.6345254785958666,
        0.6414350080393891,
        0.6484197773255048,
        0.6554806057623822,
        0.6626183215798706,
        0.6698337620266515,
        0.6771277734684463,
        0.6845012114872953,
        0.6919549409819159,
        0.6994898362691555,
        0.7071067811865475,
        0.7148066691959849,
        0.7225904034885232,
        0.7304588970903234,
        0.7384130729697496,
        0.7464538641456323,
        0.7545822137967112,
        0.762799075372269,
        0.7711054127039704,
        0.7795022001189185,
        0.7879904225539431,
        0.7965710756711334,
        0.805245165974627,
        0.8140137109286738,
        0.8228777390769823,
        0.8318382901633681,
        0.8408964152537144,
        0.8500531768592616,
        0.8593096490612387,
        0.8686669176368529,
        0.8781260801866495,
        0.8876882462632604,
        0.8973545375015533,
        0.9071260877501991,
        0.9170040432046711,
        0.9269895625416926,
        0.9370838170551498,
        0.9472879907934827,
        0.9576032806985735,
        0.9680308967461471,
        0.9785720620876999,
        0.9892280131939752,
    ],
    // Schema 7:
    &[
        0.5,
        0.5027149505564014,
        0.5054446430258502,
        0.5081891574554764,
        0.5109485743270583,
        0.5137229745593818,
        0.5165124395106142,
        0.5193170509806894,
        0.5221368912137069,
        0.5249720429003435,
        0.5278225891802786,
        0.5306886136446309,
        0.5335702003384117,
        0.5364674337629877,
        0.5393803988785598,
        0.5423091811066545,
        0.5452538663326288,
        0.5482145409081883,
        0.5511912916539204,
        0.5541842058618393,
        0.5571933712979462,
        0.5602188762048033,
        0.5632608093041209,
        0.5663192597993595,
        0.5693943173783458,
        0.572486072215902,
        0.5755946149764913,
        0.5787200368168754,
        0.5818624293887887,
        0.585021884841625,
        0.5881984958251406,
        0.5913923554921704,
        0.5946035575013605,
        0.5978321960199137,
        0.6010783657263515,
        0.6043421618132907,
        0.6076236799902344,
        0.6109230164863786,
        0.6142402680534349,
        0.6175755319684665,
        0.620928906036742,
        0.6243004885946023,
        0.6276903785123455,
        0.6310986751971253,
        0.6345254785958666,
        0.637970889198196,
        0.6414350080393891,
        0.6449179367033329,
        0.6484197773255048,
        0.6519406325959679,
        0.6554806057623822,
        0.659039800633032,
        0.6626183215798706,
        0.6662162735415805,
        0.6698337620266515,
        0.6734708931164728,
        0.6771277734684463,
        0.6808045103191123,
        0.6845012114872953,
        0.688217985377265,
        0.6919549409819159,
        0.6957121878859629,
        0.6994898362691555,
        0.7032879969095076,
        0.7071067811865475,
        0.7109463010845827,
        0.7148066691959849,
        0.718687998724491,
        0.7225904034885232,
        0.7265139979245261,
        0.7304588970903234,
        0.7344252166684908,
        0.7384130729697496,
        0.7424225829363761,
        0.7464538641456323,
        0.7505070348132126,
        0.7545822137967112,
        0.7586795205991071,
        0.762799075372269,
        0.7669409989204777,
        0.7711054127039704,
        0.7752924388424999,
        0.7795022001189185,
        0.7837348199827764,
        0.7879904225539431,
        0.7922691326262467,
        0.7965710756711334,
        0.8008963778413465,
        0.805245165974627,
        0.8096175675974316,
        0.8140137109286738,
        0.8184337248834821,
        0.8228777390769823,
        0.8273458838280969,
        0.8318382901633681,
        0.8363550898207981,
        0.8408964152537144,
        0.8454623996346523,
        0.8500531768592616,
        0.8546688815502312,
        0.8593096490612387,
        0.8639756154809185,
        0.8686669176368529,
        0.8733836930995842,
        0.8781260801866495,
        0.8828942179666361,
        0.8876882462632604,
        0.8925083056594671,
        0.8973545375015533,
        0.9022270839033115,
        0.9071260877501991,
        0.9120516927035263,
        0.9170040432046711,
        0.9219832844793128,
        0.9269895625416926,
        0.9320230241988943,
        0.9370838170551498,
        0.9421720895161669,
        0.9472879907934827,
        0.9524316709088368,
        0.9576032806985735,
        0.9628029718180622,
        0.9680308967461471,
        0.9732872087896164,
        0.9785720620876999,
        0.9838856116165875,
        0.9892280131939752,
        0.9945994234836328,
    ],
    // Schema 8:
    &[
        0.5,
        0.5013556375251013,
        0.5027149505564014,
        0.5040779490592088,
        0.5054446430258502,
        0.5068150424757447,
        0.5081891574554764,
        0.509566998038869,
        0.5109485743270583,
        0.5123338964485679,
        0.5137229745593818,
        0.5151158188430205,
        0.5165124395106142,
        0.5179128468009786,
        0.5193170509806894,
        0.520725062344158,
        0.5221368912137069,
        0.5235525479396449,
        0.5249720429003435,
        0.526395386502313,
        0.5278225891802786,
        0.5292536613972564,
        0.5306886136446309,
        0.5321274564422321,
        0.5335702003384117,
        0.5350168559101208,
        0.5364674337629877,
        0.5379219445313954,
        0.5393803988785598,
        0.5408428074966075,
        0.5423091811066545,
        0.5437795304588847,
        0.5452538663326288,
        0.5467321995364429,
        0.5482145409081883,
        0.549700901315111,
        0.5511912916539204,
        0.5526857228508706,
        0.5541842058618393,
        0.5556867516724088,
        0.5571933712979462,
        0.5587040757836845,
        0.5602188762048033,
        0.5617377836665098,
        0.5632608093041209,
        0.564787964283144,
        0.5663192597993595,
        0.5678547070789026,
        0.5693943173783458,
        0.5709381019847808,
        0.572486072215902,
        0.5740382394200894,
        0.5755946149764913,
        0.5771552102951081,
        0.5787200368168754,
        0.5802891060137493,
        0.5818624293887887,
        0.5834400184762408,
        0.585021884841625,
        0.5866080400818185,
        0.5881984958251406,
        0.5897932637314379,
        0.5913923554921704,
        0.5929957828304968,
        0.5946035575013605,
        0.5962156912915756,
        0.5978321960199137,
        0.5994530835371903,
        0.6010783657263515,
        0.6027080545025619,
        0.6043421618132907,
        0.6059806996384005,
        0.6076236799902344,
        0.6092711149137041,
        0.6109230164863786,
        0.6125793968185725,
        0.6142402680534349,
        0.6159056423670379,
        0.6175755319684665,
        0.6192499490999082,
        0.620928906036742,
        0.622612415087629,
        0.6243004885946023,
        0.6259931389331581,
        0.6276903785123455,
        0.6293922197748583,
        0.6310986751971253,
        0.6328097572894031,
        0.6345254785958666,
        0.6362458516947014,
        0.637970889198196,
        0.6397006037528346,
        0.6414350080393891,
        0.6431741147730128,
        0.6449179367033329,
        0.6466664866145447,
        0.6484197773255048,
        0.6501778216898253,
        0.6519406325959679,
        0.6537082229673385,
        0.6554806057623822,
        0.6572577939746774,
        0.659039800633032,
        0.6608266388015788,
        0.6626183215798706,
        0.6644148621029772,
        0.6662162735415805,
        0.6680225691020727,
        0.6698337620266515,
        0.6716498655934177,
        0.6734708931164728,
        0.6752968579460171,
        0.6771277734684463,
        0.6789636531064505,
        0.6808045103191123,
        0.6826503586020058,
        0.6845012114872953,
        0.6863570825438342,
        0.688217985377265,
        0.690083933630119,
        0.6919549409819159,
        0.6938310211492645,
        0.6957121878859629,
        0.6975984549830999,
        0.6994898362691555,
        0.7013863456101023,
        0.7032879969095076,
        0.7051948041086352,
        0.7071067811865475,
        0.7090239421602076,
        0.7109463010845827,
        0.7128738720527471,
        0.7148066691959849,
        0.7167447066838943,
        0.718687998724491,
        0.7206365595643126,
        0.7225904034885232,
        0.7245495448210174,
        0.7265139979245261,
        0.7284837772007218,
        0.7304588970903234,
        0.7324393720732029,
        0.7344252166684908,
        0.7364164454346837,
        0.7384130729697496,
        0.7404151139112358,
        0.7424225829363761,
        0.7444354947621984,
        0.7464538641456323,
        0.7484777058836176,
        0.7505070348132126,
        0.7525418658117031,
        0.7545822137967112,
        0.7566280937263048,
        0.7586795205991071,
        0.7607365094544071,
        0.762799075372269,
        0.7648672334736434,
        0.7669409989204777,
        0.7690203869158282,
        0.7711054127039704,
        0.7731960915705107,
        0.7752924388424999,
        0.7773944698885442,
        0.7795022001189185,
        0.7816156449856788,
        0.7837348199827764,
        0.7858597406461707,
        0.7879904225539431,
        0.7901268813264122,
        0.7922691326262467,
        0.7944171921585818,
        0.7965710756711334,
        0.7987307989543135,
        0.8008963778413465,
        0.8030678282083853,
        0.805245165974627,
        0.8074284071024302,
        0.8096175675974316,
        0.8118126635086642,
        0.8140137109286738,
        0.8162207259936375,
        0.8184337248834821,
        0.820652723822003,
        0.8228777390769823,
        0.8251087869603088,
        0.8273458838280969,
        0.8295890460808079,
        0.8318382901633681,
        0.8340936325652911,
        0.8363550898207981,
        0.8386226785089391,
        0.8408964152537144,
        0.8431763167241966,
        0.8454623996346523,
        0.8477546807446661,
        0.8500531768592616,
        0.8523579048290255,
        0.8546688815502312,
        0.8569861239649629,
        0.8593096490612387,
        0.8616394738731368,
        0.8639756154809185,
        0.8663180910111553,
        0.8686669176368529,
        0.871022112577578,
        0.8733836930995842,
        0.8757516765159389,
        0.8781260801866495,
        0.8805069215187917,
        0.8828942179666361,
        0.8852879870317771,
        0.8876882462632604,
        0.890095013257712,
        0.8925083056594671,
        0.8949281411607002,
        0.8973545375015533,
        0.8997875124702672,
        0.9022270839033115,
        0.9046732696855155,
        0.9071260877501991,
        0.909585556079304,
        0.9120516927035263,
        0.9145245157024483,
        0.9170040432046711,
        0.9194902933879467,
        0.9219832844793128,
        0.9244830347552253,
        0.9269895625416926,
        0.92950288621441,
        0.9320230241988943,
        0.9345499949706191,
        0.9370838170551498,
        0.93962450902828,
        0.9421720895161669,
        0.9447265771954693,
        0.9472879907934827,
        0.9498563490882775,
        0.9524316709088368,
        0.9550139751351947,
        0.9576032806985735,
        0.9601996065815236,
        0.9628029718180622,
        0.9654133954938133,
        0.9680308967461471,
        0.9706554947643201,
        0.9732872087896164,
        0.9759260581154889,
        0.9785720620876999,
        0.9812252401044634,
        0.9838856116165875,
        0.9865531961276168,
        0.9892280131939752,
        0.9919100824251095,
        0.9945994234836328,
        0.9972960560854698,
    ],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bound() {
        for (idx, schema, want) in vec![
            (-1, -1, 0.25),
            (0, -1, 1.0),
            (1, -1, 4.0),
            (512, -1, f64::MAX),
            (513, -1, f64::INFINITY),
            (-1, 0, 0.5),
            (0, 0, 1.0),
            (1, 0, 2.0),
            (1024, 0, f64::MAX),
            (1025, 0, f64::INFINITY),
            (-1, 2, 0.8408964152537144),
            (0, 2, 1.0),
            (1, 2, 1.189207115002721),
            (4096, 2, f64::MAX),
            (4097, 2, f64::INFINITY),
        ] {
            let got = get_bound(idx, schema);
            assert_eq!(want, got, "idx {}, schema {}", idx, schema);
        }
    }
}
