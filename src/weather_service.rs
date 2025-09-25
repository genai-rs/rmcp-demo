use anyhow::Result;
use opentelemetry::trace::{FutureExt, TraceContextExt, Tracer};
use opentelemetry::{global, KeyValue};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Serialize, Deserialize)]
pub struct Weather {
    pub location: String,
    pub temperature: i32,
    pub condition: String,
    pub humidity: i32,
    pub wind_speed: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Forecast {
    pub day: i32,
    pub high: i32,
    pub low: i32,
    pub condition: String,
    pub precipitation_chance: i32,
}

pub struct WeatherService {
    tracer: global::BoxedTracer,
}

impl WeatherService {
    pub fn new() -> Self {
        Self {
            tracer: global::tracer("weather-service"),
        }
    }

    #[instrument(skip(self))]
    pub async fn get_weather(&self, location: &str) -> Result<Weather> {
        let span = self
            .tracer
            .span_builder("get_weather")
            .with_attributes(vec![
                KeyValue::new("location", location.to_string()),
                KeyValue::new("service", "weather"),
            ])
            .start(&self.tracer);

        let cx = opentelemetry::Context::current_with_span(span);

        async move {
            let mut rng = rand::thread_rng();
            let weather_conditions = ["Sunny", "Cloudy", "Rainy", "Partly Cloudy"];

            let weather = Weather {
                location: location.to_string(),
                temperature: rng.gen_range(15..=30),
                condition: weather_conditions[rng.gen_range(0..weather_conditions.len())]
                    .to_string(),
                humidity: rng.gen_range(40..=80),
                wind_speed: rng.gen_range(5..=25),
            };

            Ok(weather)
        }
        .with_context(cx)
        .await
    }

    #[instrument(skip(self))]
    pub async fn get_forecast(&self, location: &str, days: usize) -> Result<Vec<Forecast>> {
        let span = self
            .tracer
            .span_builder("get_forecast")
            .with_attributes(vec![
                KeyValue::new("location", location.to_string()),
                KeyValue::new("days", days as i64),
                KeyValue::new("service", "weather"),
            ])
            .start(&self.tracer);

        let cx = opentelemetry::Context::current_with_span(span);

        async move {
            let mut rng = rand::thread_rng();
            let conditions = ["Sunny", "Cloudy", "Rainy", "Stormy"];

            let forecast: Vec<Forecast> = (1..=days.min(7))
                .map(|day| Forecast {
                    day: day as i32,
                    high: rng.gen_range(20..=35),
                    low: rng.gen_range(10..=20),
                    condition: conditions[rng.gen_range(0..conditions.len())].to_string(),
                    precipitation_chance: rng.gen_range(0..=100),
                })
                .collect();

            Ok(forecast)
        }
        .with_context(cx)
        .await
    }
}
