#[cfg(test)]
mod tests {
    use crate::engine::*;
    use crate::platform;
    use crate::units;
    use crate::units::Date;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

    #[test]
    fn menu_selection_leaves_separator_clear_at_each_dpi() {
        use crate::ui::{metrics, Rect};
        for scale in [1.0_f32, 1.25, 1.5, 2.0] {
            let dpi = (96.0 * scale) as u32;
            let fonts = crate::ui::Fonts::new(dpi);
            let hit = Rect::new(9, 0, 100, platform::menu_bar_height(dpi, fonts.body_height));
            let button = crate::app::menu_button_rect(hit, scale);
            let separator_y = hit.bottom() - 1;
            assert_eq!(
                separator_y - button.bottom(),
                (metrics::MENU_FOOT_GAP * scale).round() as i32
            );
            assert_eq!((button.x, button.w), (hit.x, hit.w));
            assert_eq!(button.y - hit.y, separator_y - button.bottom());
            assert!(button.h > 0);
            assert!(hit.contains(hit.x + 1, separator_y - 1));
            assert!(!button.contains(button.x + 1, separator_y));
        }
    }

    #[test]
    fn mode_accelerators_cover_every_view() {
        let has = |key: u16, cmd: u16| {
            platform::MODE_ACCELERATORS
                .iter()
                .any(|&(k, c)| k == key && c == cmd)
        };
        assert!(has(VK_1, platform::MODE_STANDARD));
        assert!(has(VK_2, platform::MODE_SCIENTIFIC));
        assert!(has(VK_3, platform::MODE_PROGRAMMER));
        assert!(has(VK_4, platform::MODE_STATISTICS));
        assert!(has(VK_5, platform::MODE_UNIT));
        assert!(has(VK_6, platform::MODE_DATE));
        assert!(has(VK_7, platform::MODE_MORTGAGE));
        assert!(has(VK_8, platform::MODE_LEASE));
        assert!(has(VK_9, platform::MODE_MPG));
        assert!(has(VK_0, platform::MODE_L100));
        assert!(has(VK_U, platform::MODE_UNIT));
        assert!(has(VK_E, platform::MODE_DATE));
    }

    fn press(e: &mut StandardEngine, text: &str) {
        for ch in text.chars() {
            match ch {
                '0'..='9' => e.digit(ch),
                '.' => e.decimal(),
                '+' => e.push_op(Op::Add),
                '-' => e.push_op(Op::Sub),
                '*' => e.push_op(Op::Mul),
                '/' => e.push_op(Op::Div),
                '^' => e.push_op(Op::Pow),
                '=' => e.equals(),
                '(' => e.open_paren(),
                ')' => e.close_paren(),
                '%' => e.percent(),
                'c' => e.clear_all(),
                _ => {}
            }
        }
    }

    fn value(e: &StandardEngine) -> f64 {
        parse_number(&e.display(false)).unwrap_or(f64::NAN)
    }

    #[test]
    fn standard_arithmetic() {
        let mut e = StandardEngine::new();
        press(&mut e, "2+2=");
        assert!((value(&e) - 4.0).abs() < 1e-12);
        press(&mut e, "5*3=");
        assert!((value(&e) - 15.0).abs() < 1e-12);
        press(&mut e, "10/4=");
        assert!((value(&e) - 2.5).abs() < 1e-12);
        press(&mut e, "6-9=");
        assert!((value(&e) + 3.0).abs() < 1e-12);
    }

    #[test]
    fn precedence_matches_original() {
        let mut e = StandardEngine::new();
        press(&mut e, "2+3*4=");
        assert!((value(&e) - 14.0).abs() < 1e-12);
    }

    #[test]
    fn repeat_equals_matches_original() {
        let mut e = StandardEngine::new();
        press(&mut e, "2+3=");
        assert!((value(&e) - 5.0).abs() < 1e-12);
        e.equals();
        assert!((value(&e) - 8.0).abs() < 1e-12);
        e.equals();
        assert!((value(&e) - 11.0).abs() < 1e-12);

        let mut e = StandardEngine::new();
        press(&mut e, "2+3*4=");
        assert!((value(&e) - 14.0).abs() < 1e-12);
        e.equals();
        assert!((value(&e) - 26.0).abs() < 1e-12);
        e.equals();
        assert!((value(&e) - 38.0).abs() < 1e-12);

        // A number typed after `=` becomes the left side of the last
        // operation: `2 + 3 = 4 =` -> 7.
        let mut e = StandardEngine::new();
        press(&mut e, "2+3=4=");
        assert!((value(&e) - 7.0).abs() < 1e-12);
        e.equals();
        assert!((value(&e) - 10.0).abs() < 1e-12);

        // With no trailing operator there is nothing to repeat.
        let mut e = StandardEngine::new();
        press(&mut e, "5=");
        assert!((value(&e) - 5.0).abs() < 1e-12);
        e.equals();
        assert!((value(&e) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn repeat_equals_uses_last_evaluated_operation() {
        let cases = [
            ("2+3*4===", 38.0),
            ("2-3*4==", -22.0),
            ("2*3+4==", 14.0),
            ("10-2-3==", 2.0),
            ("2*3*4==", 96.0),
            ("2+3*4*5==", 122.0),
            ("2*3+4*5==", 46.0),
            ("100/10/2==", 2.5),
            ("9*9=+1==", 83.0),
            ("9*9=-1==", 79.0),
            ("2+3=+4===", 17.0),
        ];
        for (text, expected) in cases {
            let mut e = StandardEngine::new();
            press(&mut e, text);
            assert!(
                (value(&e) - expected).abs() < 1e-9,
                "{text}: expected {expected}, got {}",
                value(&e)
            );
        }
    }

    #[test]
    fn equals_uses_display_after_trailing_operator() {
        // The missing operand is the value the original shows after the
        // trailing operator, not blindly the last typed number.
        let cases = [
            ("2+=", 4.0),
            ("2+3+=", 10.0),
            ("2*3+=", 12.0),
            ("2+3*=", 11.0),
            ("2+3/=", 3.0),
            ("2+3-=", 0.0),
            ("2*3*=", 36.0),
            ("2+3*4*=", 146.0),
            ("2+3*4+=", 28.0),
            ("2*3*4+=", 48.0),
            ("2+3*4/=", 3.0),
            ("2+3*===", 29.0),
            ("2+3*4*===", 434.0),
            ("2+3*4+===", 56.0),
        ];
        for (text, expected) in cases {
            let mut e = StandardEngine::new();
            press(&mut e, text);
            assert!(
                (value(&e) - expected).abs() < 1e-9,
                "{text}: expected {expected}, got {}",
                value(&e)
            );
        }
    }

    #[test]
    fn repeat_equals_with_parentheses_and_powers() {
        let cases = [
            ("2*(3+4)*=", 196.0),
            ("(2+3)*4*=", 400.0),
            ("2+3*(4+5)*=", 731.0),
            ("2*(3+4)*==", 2744.0),
            ("(2+3)*4*==", 8000.0),
            ("2+3*(4+5)*==", 1460.0),
            ("2^3^2=", 64.0),
            ("2^3^=", 16777216.0),
            ("16 yroot 2 yroot 2 =", 2.0),
        ];
        for (text, expected) in cases {
            let mut e = StandardEngine::new();
            if text.contains("yroot") {
                press(&mut e, "16");
                e.push_op(Op::Root);
                press(&mut e, "2");
                e.push_op(Op::Root);
                press(&mut e, "2=");
            } else {
                press(&mut e, text);
            }
            assert!(
                (value(&e) - expected).abs() < 1e-9,
                "{text}: expected {expected}, got {}",
                value(&e)
            );
        }
    }

    #[test]
    fn clear_entry_keeps_pending_and_repeat() {
        let mut e = StandardEngine::new();
        press(&mut e, "2+3");
        e.clear_entry();
        assert_eq!(e.display(false), "0");
        e.equals();
        assert!((value(&e) - 2.0).abs() < 1e-12);

        let mut e = StandardEngine::new();
        press(&mut e, "2+3=");
        e.clear_entry();
        assert_eq!(e.display(false), "0");
        e.equals();
        assert!((value(&e) - 3.0).abs() < 1e-12);

        let mut e = StandardEngine::new();
        press(&mut e, "2+3=");
        e.clear_entry();
        press(&mut e, "5=");
        assert!((value(&e) - 8.0).abs() < 1e-12);

        // C discards the repeat operation.
        let mut e = StandardEngine::new();
        press(&mut e, "2+3=");
        e.clear_all();
        e.equals();
        assert_eq!(value(&e), 0.0);
    }

    #[test]
    fn expression_line_shows_committed_tokens_only() {
        let mut e = StandardEngine::new();
        press(&mut e, "2+3");
        assert_eq!(e.expression(false), "2 +");
        press(&mut e, "*4");
        assert_eq!(e.expression(false), "2 + 3 ×");
        press(&mut e, "=");
        assert_eq!(e.expression(false), "");

        let mut e = StandardEngine::new();
        press(&mut e, "2+3");
        e.clear_entry();
        assert_eq!(e.expression(false), "2 +");
        assert_eq!(e.display(false), "0");
    }

    #[test]
    fn percent_repeat_matches_original() {
        let mut e = StandardEngine::new();
        press(&mut e, "200+10%=");
        assert!((value(&e) - 220.0).abs() < 1e-9);
        e.equals();
        assert!((value(&e) - 240.0).abs() < 1e-9);
    }

    #[test]
    fn division_by_zero_messages() {
        let mut e = StandardEngine::new();
        press(&mut e, "1/0=");
        assert_eq!(e.display(false), "Cannot divide by zero");
        let mut e = StandardEngine::new();
        press(&mut e, "0/0=");
        assert_eq!(e.display(false), "Result is undefined");
    }

    #[test]
    fn display_precision_is_sixteen_digits() {
        let mut e = StandardEngine::new();
        press(&mut e, "1/3=");
        assert_eq!(e.display(false), "0,3333333333333333");
        let mut e = StandardEngine::new();
        press(&mut e, "1/300=");
        assert_eq!(e.display(false), "0,0033333333333333");
        let mut e = StandardEngine::new();
        for ch in "12345678901234567".chars() {
            e.digit(ch);
        }
        assert_eq!(e.display(false), "1234567890123456");
    }

    #[test]
    fn percent_matches_original() {
        let mut e = StandardEngine::new();
        press(&mut e, "200+10");
        e.percent();
        e.equals();
        assert!((value(&e) - 220.0).abs() < 1e-9);

        let mut e = StandardEngine::new();
        press(&mut e, "200*10");
        e.percent();
        e.equals();
        assert!((value(&e) - 4000.0).abs() < 1e-9);

        let mut e = StandardEngine::new();
        press(&mut e, "200/10");
        e.percent();
        e.equals();
        assert!((value(&e) - 10.0).abs() < 1e-9);

        let mut e = StandardEngine::new();
        press(&mut e, "50");
        e.percent();
        e.equals();
        assert_eq!(value(&e), 0.0);
    }

    #[test]
    fn scientific_values() {
        assert!((apply_angle(30.0, AngleMode::Degrees, false, "sin").unwrap() - 0.5).abs() < 1e-12);
        assert!((apply_angle(60.0, AngleMode::Degrees, false, "cos").unwrap() - 0.5).abs() < 1e-12);
        assert!((apply_angle(45.0, AngleMode::Degrees, false, "tan").unwrap() - 1.0).abs() < 1e-12);
        assert!((factorial(5.0).unwrap() - 120.0).abs() < 1e-12);
    }

    #[test]
    fn scientific_display_format() {
        assert_eq!(format_scientific(1234.5, false), "1,2345e+3");
        assert_eq!(format_scientific(0.0, false), "0e+0");
        assert_eq!(format_scientific(-0.00125, false), "−1,25e−3");
    }

    #[test]
    fn programmer_operations() {
        let mut p = ProgrammerState::new();
        p.set_base(10);
        p.digit('5');
        p.push_op(ProgOp::Or);
        p.digit('2');
        p.equals();
        assert_eq!(p.value, 7);
        p.clear_all();
        p.digit('5');
        p.push_op(ProgOp::And);
        p.digit('3');
        p.equals();
        assert_eq!(p.value, 1);
        p.clear_all();
        p.digit('5');
        p.push_op(ProgOp::Xor);
        p.digit('3');
        p.equals();
        assert_eq!(p.value, 6);
        p.clear_all();
        p.digit('1');
        p.push_op(ProgOp::Lsh);
        p.digit('4');
        p.equals();
        assert_eq!(p.value, 16);
    }

    fn prog_press(p: &mut ProgrammerState, text: &str) {
        for ch in text.chars() {
            match ch {
                '0'..='9' => p.digit(ch),
                '+' => p.push_op(ProgOp::Add),
                '-' => p.push_op(ProgOp::Sub),
                '*' => p.push_op(ProgOp::Mul),
                '/' => p.push_op(ProgOp::Div),
                '%' => p.push_op(ProgOp::Mod),
                '|' => p.push_op(ProgOp::Or),
                '&' => p.push_op(ProgOp::And),
                '^' => p.push_op(ProgOp::Xor),
                '<' => p.push_op(ProgOp::Lsh),
                '>' => p.push_op(ProgOp::Rsh),
                '=' => p.equals(),
                '~' => p.not(),
                's' => p.sqrt(),
                'r' => p.reciprocal(),
                'n' => p.toggle_sign(),
                '!' => p.clear_entry(),
                _ => {}
            }
        }
    }

    #[test]
    fn programmer_repeat_equals_and_precedence_match_original() {
        let cases = [
            ("2+3===", 11u64),
            ("2+3*4=", 14),
            ("2+3*4==", 26),
            ("2+3=4=", 7),
            ("2+3+=", 10),
            ("2+3*=", 11),
            ("2*3+=", 12),
            ("2+3+*4=", 20),
            ("2+3*+4=", 9),
            ("2+3+/4=", 1),
            ("2+3+-4=", 1),
            ("2+3 s=", 3),
            ("2+3 s+=", 6),
            ("1|2^3=", 0),
            ("2|1&4=", 2),
            ("4^1&1=", 5),
            ("1<2*2=", 8),
            ("8/2<1=", 8),
            ("2+1<1=", 4),
            ("1<1+2=", 4),
        ];
        for (text, expected) in cases {
            let mut p = ProgrammerState::new();
            prog_press(&mut p, text);
            assert_eq!(p.value, expected, "{text}");
        }

        let mut p = ProgrammerState::new();
        prog_press(&mut p, "2+3 n=");
        assert_eq!(p.display(), "−1");
        let mut p = ProgrammerState::new();
        prog_press(&mut p, "2+3 ~=");
        assert_eq!(p.display(), "−2");
        let mut p = ProgrammerState::new();
        prog_press(&mut p, "2+3*");
        assert_eq!(p.expression(), "");
    }

    #[test]
    fn programmer_clear_entry_keeps_pending_and_repeat() {
        let mut p = ProgrammerState::new();
        prog_press(&mut p, "2+3");
        p.clear_entry();
        assert_eq!(p.display(), "0");
        p.equals();
        assert_eq!(p.value, 2);

        let mut p = ProgrammerState::new();
        prog_press(&mut p, "2+3=");
        p.clear_entry();
        assert_eq!(p.display(), "0");
        p.equals();
        assert_eq!(p.value, 3);

        let mut p = ProgrammerState::new();
        prog_press(&mut p, "2+3=");
        p.clear_entry();
        prog_press(&mut p, "5=");
        assert_eq!(p.value, 8);
    }

    #[test]
    fn programmer_qword_accepts_nineteen_decimal_digits() {
        let mut p = ProgrammerState::new();
        for _ in 0..19 {
            p.digit('7');
        }
        assert_eq!(p.value, 7_777_777_777_777_777_777);
        assert_eq!(p.display(), "7 777 777 777 777 777 777");
        // A twentieth digit would leave the unsigned 64-bit range.
        p.digit('7');
        assert_eq!(p.value, 7_777_777_777_777_777_777);
    }

    #[test]
    fn programmer_grouping_matches_base_width() {
        assert_eq!(
            format_prog(0x1234_5678_9abc_def0, 16, 64),
            "1234 5678 9ABC DEF0"
        );
        assert_eq!(format_prog(0o777_777, 8, 64), "777 777");
        assert_eq!(format_prog(0b1111_0000, 2, 64), "1111 0000");
        assert_eq!(format_prog(1_234_567, 10, 64), "1 234 567");
    }

    #[test]
    fn programmer_qword_accepts_full_binary_word() {
        let mut p = ProgrammerState::new();
        p.set_base(2);
        for _ in 0..64 {
            p.digit('1');
        }
        assert_eq!(p.value, u64::MAX);
        assert_eq!(p.display().chars().filter(|ch| *ch == '1').count(), 64);
    }

    #[test]
    fn statistics_formulas() {
        let mut s = StatsState::new();
        for v in [1.0, 2.0, 3.0] {
            s.recall(v);
            s.add_data();
        }
        assert_eq!(s.count(), 3);
        assert!((s.sum().unwrap() - 6.0).abs() < 1e-12);
        assert!((s.mean().unwrap() - 2.0).abs() < 1e-12);
        assert!((s.sum_squares().unwrap() - 14.0).abs() < 1e-12);
        assert!((s.mean_squares().unwrap() - 14.0 / 3.0).abs() < 1e-12);
        assert!((s.std_dev_population().unwrap() - (2.0f64 / 3.0).sqrt()).abs() < 1e-12);
        assert!((s.std_dev_sample().unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn unit_conversion_values() {
        let cats = units::categories();
        let length = &cats[0];
        let inch = length.units.iter().find(|u| u.name == "Inch").unwrap();
        let cm = length
            .units
            .iter()
            .find(|u| u.name == "Centimeters")
            .unwrap();
        assert!((units::convert(1.0, inch, cm) - 2.54).abs() < 1e-12);
        let mile = length.units.iter().find(|u| u.name == "Mile").unwrap();
        assert!((units::convert(1.0, mile, cm) - 160934.4).abs() < 1e-9);
        let rod = length.units.iter().find(|u| u.name == "Rods").unwrap();
        assert!((units::convert(1.0, rod, cm) - 502.92).abs() < 1e-9);
        let pica = length.units.iter().find(|u| u.name == "PICA").unwrap();
        assert!((units::convert(1.0, pica, cm) - 0.42175176).abs() < 1e-12);
        let temp = &cats[3];
        let c = temp
            .units
            .iter()
            .find(|u| u.name == "Degrees Celsius")
            .unwrap();
        let f = temp
            .units
            .iter()
            .find(|u| u.name == "Degrees Fahrenheit")
            .unwrap();
        assert!((units::convert(100.0, c, f) - 212.0).abs() < 1e-9);
    }

    #[test]
    fn dcs_and_process_conversions() {
        fn unit<'a>(category: &'a units::Category, name: &str) -> &'a units::Unit {
            category.units.iter().find(|u| u.name == name).unwrap()
        }

        let cats = units::categories();
        let category = |name: &str| cats.iter().find(|c| c.name == name).unwrap();

        let signals = category("DCS Signals");
        let percent = unit(signals, "Percent of span (%)");
        let ma = unit(signals, "Current (4-20 mA)");
        let volts = unit(signals, "Voltage (1-5 V)");
        let raw = unit(signals, "Siemens raw (0-27648)");
        assert!(units::convert(4.0, ma, percent).abs() < 1e-9);
        assert!((units::convert(20.0, ma, percent) - 100.0).abs() < 1e-9);
        assert!((units::convert(50.0, percent, ma) - 12.0).abs() < 1e-9);
        assert!(units::convert(1.0, volts, percent).abs() < 1e-9);
        assert!((units::convert(5.0, volts, percent) - 100.0).abs() < 1e-9);
        assert!((units::convert(27_648.0, raw, percent) - 100.0).abs() < 1e-9);
        assert!((units::convert(100.0, percent, raw) - 27_648.0).abs() < 1e-9);

        let pressure = category("Pressure");
        let mpa = unit(pressure, "Mega Pascal");
        let bar = unit(pressure, "Bar");
        let kgf_cm2 = unit(pressure, "Kilogram-force per square centimeter");
        let mm_h2o = unit(pressure, "Millimeter of water");
        let pascal = unit(pressure, "Pascal");
        assert!((units::convert(1.0, mpa, bar) - 10.0).abs() < 1e-12);
        assert!((units::convert(1.0, kgf_cm2, bar) - 0.980665).abs() < 1e-12);
        assert!((units::convert(1.0, mm_h2o, pascal) - 9.80665).abs() < 1e-12);

        let flow = category("Flow");
        let m3h = unit(flow, "Cubic meter per hour");
        let lmin = unit(flow, "Liter per minute");
        let gpm = unit(flow, "US gallon per minute (GPM)");
        assert!((units::convert(1.0, m3h, lmin) - 16.666666666666668).abs() < 1e-9);
        assert!((units::convert(1.0, gpm, m3h) - 0.22712470704).abs() < 1e-9);

        let mass = category("Mass Flow");
        let tonne_h = unit(mass, "Tonne per hour");
        let kg_s = unit(mass, "Kilogram per second");
        assert!((units::convert(1.0, tonne_h, kg_s) - 0.2777777777777778).abs() < 1e-12);

        let force = category("Force");
        let kgf = unit(force, "Kilogram-force");
        let newton = unit(force, "Newton");
        assert!((units::convert(1.0, kgf, newton) - 9.80665).abs() < 1e-12);
        let torque = category("Torque");
        let kgf_m = unit(torque, "Kilogram-force meter");
        let n_m = unit(torque, "Newton meter");
        assert!((units::convert(1.0, kgf_m, n_m) - 9.80665).abs() < 1e-12);

        let density = category("Density");
        let sg = unit(density, "Specific gravity (water = 1)");
        let kg_m3 = unit(density, "Kilogram per cubic meter");
        assert!((units::convert(1.0, sg, kg_m3) - 1000.0).abs() < 1e-12);

        let rotation = category("Rotational Speed");
        let rpm = unit(rotation, "Revolution per minute (rpm)");
        let rad_s = unit(rotation, "Radian per second");
        assert!((units::convert(1.0, rpm, rad_s) - 0.10471975511965977).abs() < 1e-12);

        let energy = category("Energy");
        let kwh = unit(energy, "Kilowatt-hour");
        let joule = unit(energy, "Joule");
        assert!((units::convert(1.0, kwh, joule) - 3_600_000.0).abs() < 1e-6);

        let loop24 = units::loop_resistor(24.0, 12.0).unwrap();
        assert!((loop24.resistance - 2_000.0).abs() < 1e-9);
        assert!((loop24.power - 0.288).abs() < 1e-12);
        assert!((loop24.standard_resistance - 2_000.0).abs() < 1e-9);
        assert!((loop24.actual_current_ma - 12.0).abs() < 1e-9);
        assert!((loop24.recommended_wattage - 0.5).abs() < 1e-12);
        assert_eq!(units::format_resistance(2_000.0, true), "2 kΩ");
        let loop4 = units::loop_resistor(24.0, 4.0).unwrap();
        assert!((loop4.standard_resistance - 6_200.0).abs() < 1e-9);
        assert!(units::loop_resistor(24.0, 0.0).is_none());
        assert!(units::loop_resistor(0.0, 12.0).is_none());

        assert_eq!(category("Volume").units[0].name, "Cubic centimeter");
        assert_eq!(category("Velocity").units[0].name, "Centimeter per second");
        assert_eq!(category("Power").units[0].name, "Watt");

        for category in &cats {
            assert!(category.units.len() >= 2, "{}", category.name);
            let mut names = std::collections::HashSet::new();
            for unit in &category.units {
                assert!(unit.factor.is_finite() && unit.factor != 0.0);
                assert!(unit.offset.is_finite());
                assert!(names.insert(unit.name), "duplicate {}", unit.name);
            }
        }
    }

    #[test]
    fn mortgage_formula() {
        let values = [100_000.0, 0.0, 30.0, 6.0, f64::NAN];
        let payment = units::solve_worksheet(WorksheetKind::Mortgage, 4, &values).unwrap();
        assert!((payment - 599.5505251527569).abs() < 1e-6, "{payment}");
    }

    #[test]
    fn lease_formula() {
        let values = [30_000.0, 3.0, 12.0, 15_000.0, 5.0, f64::NAN];
        let payment = units::solve_worksheet(WorksheetKind::VehicleLease, 5, &values).unwrap();
        assert!((payment - 512.0634565699831).abs() < 1e-6, "{payment}");
    }

    #[test]
    fn fuel_economy_formulas() {
        let mpg =
            units::solve_worksheet(WorksheetKind::FuelMpg, 2, &[300.0, 10.0, f64::NAN]).unwrap();
        assert!((mpg - 30.0).abs() < 1e-12);
        let l100 =
            units::solve_worksheet(WorksheetKind::FuelL100, 2, &[500.0, 40.0, f64::NAN]).unwrap();
        assert!((l100 - 8.0).abs() < 1e-12);
    }

    #[test]
    fn date_difference_and_shift() {
        let a = Date::new(2026, 11, 2).unwrap();
        let b = Date::new(2027, 12, 5).unwrap();
        let (y, m, w, d, total) = units::date_difference(a, b);
        assert_eq!((y, m), (1, 1));
        assert!(total > 0);
        assert_eq!(i64::from(w * 7 + d), total - 365 - 30);
        let shifted = units::shift_date(a, 1, 1, 3, true).unwrap();
        assert_eq!(shifted, Date::new(2027, 12, 5).unwrap());
        assert_eq!(
            units::parse_date("02.11.2026"),
            Some(Date::new(2026, 11, 2).unwrap())
        );
    }

    #[test]
    fn date_result_text_matches_original() {
        assert_eq!(crate::date_parts(1, 1, 0, 1), "1 year, 1 month, 1 day");
        assert_eq!(crate::date_parts(0, 0, 0, 0), "Same dates");
    }

    #[test]
    fn keypad_geometry_is_shared() {
        use crate::actions::{standard_keys, KeyStyle};
        use crate::ui::{key_cells, Rect};

        let keys = standard_keys();
        let cells = key_cells(Rect::new(0, 0, 287, 286), &keys, 5, 6, 8);
        assert_eq!(cells.len(), keys.len());
        let one = cells.iter().find(|(_, key)| key.label == "1").unwrap();
        let zero = cells.iter().find(|(_, key)| key.label == "0").unwrap();
        let equals = cells
            .iter()
            .find(|(_, key)| key.style == KeyStyle::Equals)
            .unwrap();
        assert!(zero.0.w > one.0.w, "zero spans two columns");
        assert!(equals.0.h > one.0.h, "equals spans two rows");
    }

    #[test]
    fn all_keypads_use_same_cell_size() {
        use crate::actions::{programmer_keys, standard_keys, stats_keys};
        use crate::ui::{key_cells, metrics, Rect};

        let standard_keys = standard_keys();
        let programmer_keys = programmer_keys();
        let stats_keys = stats_keys("Add");
        let standard = key_cells(
            Rect::new(0, 0, metrics::KEYPAD_W as i32, metrics::KEYPAD_H as i32),
            &standard_keys,
            metrics::KEYPAD_COLS,
            metrics::KEYPAD_ROWS,
            metrics::KEY_GAP as i32,
        );
        let programmer = key_cells(
            Rect::new(
                0,
                0,
                metrics::KEYPAD_WIDE_W as i32,
                metrics::KEYPAD_H as i32,
            ),
            &programmer_keys,
            metrics::KEYPAD_COLS_WIDE,
            metrics::KEYPAD_ROWS,
            metrics::KEY_GAP as i32,
        );
        let statistics = key_cells(
            Rect::new(0, 0, metrics::SIDEBAR_W as i32, metrics::KEYPAD_H as i32),
            &stats_keys,
            metrics::KEYPAD_COLS,
            metrics::KEYPAD_ROWS,
            metrics::KEY_GAP as i32,
        );

        let size = |cells: &[(Rect, &crate::actions::KeySpec)]| {
            let (rect, _) = cells[0];
            (rect.w, rect.h)
        };
        let expected = (metrics::KEY_CELL_W as i32, metrics::KEY_CELL_H as i32);
        assert_eq!(size(&standard), expected);
        assert_eq!(size(&programmer), expected);
        assert_eq!(size(&statistics), expected);
    }

    #[test]
    fn programmer_digit_availability_follows_base() {
        use crate::engine::digit_allowed;

        for ch in "0123456789ABCDEF".chars() {
            assert!(digit_allowed(16, ch), "hex should allow {ch}");
        }
        assert!(digit_allowed(10, '9'));
        assert!(!digit_allowed(10, 'A'));
        assert!(digit_allowed(8, '7'));
        assert!(!digit_allowed(8, '8'));
        assert!(digit_allowed(2, '1'));
        assert!(!digit_allowed(2, '2'));
    }

    #[test]
    fn localization_switches_russian_and_english() {
        use crate::i18n::{category, date_parts, unit, Lang};
        use crate::ui::palette::{ThemeMode, DARK, LIGHT};

        assert_eq!(Lang::default(), Lang::Ru);
        assert_eq!(date_parts(Lang::Ru, 1, 1, 0, 1), "1 год, 1 месяц, 1 день");
        assert_eq!(date_parts(Lang::En, 1, 1, 0, 1), "1 year, 1 month, 1 day");
        assert_eq!(date_parts(Lang::Ru, 0, 0, 0, 0), "Даты совпадают");
        assert_eq!(unit(Lang::Ru, "Meter"), "Метр");
        assert_eq!(unit(Lang::En, "Meter"), "Meter");
        assert_eq!(category(Lang::Ru, "DCS Signals"), "Сигналы DCS");
        assert_eq!(category(Lang::Ru, "Torque"), "Крутящий момент");
        assert_eq!(unit(Lang::Ru, "Current (4-20 mA)"), "Ток (4–20 мА)");
        assert_eq!(Lang::Ru.text().dcs_loop_title, "Подбор сопротивления");
        assert_eq!(Lang::En.text().dcs_loop_title, "Loop resistor");
        assert_eq!(ThemeMode::System.resolve(false).bg_top, LIGHT.bg_top);
        assert_eq!(ThemeMode::System.resolve(true).bg_top, DARK.bg_top);
    }
}
