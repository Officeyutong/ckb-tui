use anyhow::bail;
use cursive::View;

pub struct SimpleBarChart {
    data: Vec<f64>,
    max_value: f64,
}

const STEP: f64 = 0.125;

impl SimpleBarChart {
    pub fn set_data(&mut self, new_data: &[f64]) -> anyhow::Result<()> {
        if new_data.iter().any(|x| *x < 0.0 || *x > 1.0) {
            bail!("Invalid data, all numbers must be in range [0,1]");
        }
        self.data = new_data.to_vec();
        Ok(())
    }
    pub fn set_max_value(&mut self, max_value: f64) {
        self.max_value = max_value;
    }
    pub fn new(data: &[f64]) -> anyhow::Result<Self> {
        let mut new_inst = Self {
            data: Default::default(),
            max_value: 1.0,
        };
        new_inst.set_data(data)?;
        Ok(new_inst)
    }
}

impl View for SimpleBarChart {
    fn draw(&self, printer: &cursive::Printer) {
        let mut str = String::default();
        for item in self.data.iter() {
            let item = *item / self.max_value;
            let char = if item == STEP * 0.0 {
                ' '
            } else if item > STEP * 0.0 && item <= STEP * 1.0 {
                '▁'
            } else if item > STEP * 1.0 && item <= STEP * 2.0 {
                '▂'
            } else if item > STEP * 2.0 && item <= STEP * 3.0 {
                '▃'
            } else if item > STEP * 3.0 && item <= STEP * 4.0 {
                '▄'
            } else if item > STEP * 4.0 && item <= STEP * 5.0 {
                '▅'
            } else if item > STEP * 5.0 && item <= STEP * 6.0 {
                '▆'
            } else if item > STEP * 6.0 && item <= STEP * 7.0 {
                '▇'
            } else if item > STEP * 7.0 && item <= STEP * 8.0 {
                '█'
            } else {
                unreachable!()
            };
            str.push(char);
        }
        printer.print((0, 0), &str);
    }
    fn required_size(&mut self, _constraint: cursive::Vec2) -> cursive::Vec2 {
        (self.data.len(), 1).into()
    }
}
