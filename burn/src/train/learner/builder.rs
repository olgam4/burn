use super::Learner;
use crate::module::ADModule;
use crate::train::checkpoint::{AsyncCheckpointer, Checkpointer, FileCheckpointer};
use crate::train::logger::FileMetricLogger;
use crate::train::metric::dashboard::cli::CLIDashboardRenderer;
use crate::train::metric::dashboard::Dashboard;
use crate::train::metric::{Metric, Numeric};
use crate::train::AsyncTrainerCallback;
use burn_tensor::backend::ADBackend;
use burn_tensor::Element;
use std::sync::Arc;

/// Struct to configure and create a [learner](Learner).
pub struct LearnerBuilder<B, T, V>
where
    T: Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: ADBackend,
{
    dashboard: Dashboard<T, V>,
    checkpointer_model: Option<Arc<dyn Checkpointer<B::Elem> + Send + Sync>>,
    checkpointer_optimizer: Option<Arc<dyn Checkpointer<B::Elem> + Send + Sync>>,
    num_epochs: usize,
    checkpoint: Option<usize>,
    directory: String,
}

impl<B, T, V> LearnerBuilder<B, T, V>
where
    T: Send + Sync + 'static,
    V: Send + Sync + 'static,
    B: ADBackend,
{
    pub fn new(directory: &str) -> Self {
        let renderer = Box::new(CLIDashboardRenderer::new());
        let logger_train = Box::new(FileMetricLogger::new(
            format!("{}/train", directory).as_str(),
        ));
        let logger_valid = Box::new(FileMetricLogger::new(
            format!("{}/valid", directory).as_str(),
        ));

        Self {
            dashboard: Dashboard::new(renderer, logger_train, logger_valid),
            num_epochs: 1,
            checkpoint: None,
            checkpointer_model: None,
            checkpointer_optimizer: None,
            directory: directory.to_string(),
        }
    }

    /// Register a training metric.
    pub fn metric_train<M: Metric<T> + 'static>(mut self, metric: M) -> Self {
        self.dashboard.register_train(metric);
        self
    }

    /// Register a validation metric.
    pub fn metric_valid<M: Metric<V> + 'static>(mut self, metric: M) -> Self {
        self.dashboard.register_valid(metric);
        self
    }

    /// Register a training metric and displays it on a plot.
    ///
    /// # Notes
    ///
    /// Only [numeric](Numeric) metric can be displayed on a plot.
    /// If the same metric is also registered for the [validation split](Self::metric_valid_plot),
    /// the same graph will be used for both.
    pub fn metric_train_plot<M: Metric<T> + Numeric + 'static>(mut self, metric: M) -> Self {
        self.dashboard.register_train_plot(metric);
        self
    }

    /// Register a validation metric and displays it on a plot.
    ///
    /// # Notes
    ///
    /// Only [numeric](Numeric) metric can be displayed on a plot.
    /// If the same metric is also registered for the [training split](Self::metric_train_plot),
    /// the same graph will be used for both.
    pub fn metric_valid_plot<M: Metric<V> + Numeric + 'static>(mut self, metric: M) -> Self {
        self.dashboard.register_valid_plot(metric);
        self
    }

    /// The number of epochs the training should last.
    pub fn num_epochs(mut self, num_epochs: usize) -> Self {
        self.num_epochs = num_epochs;
        self
    }

    /// The epoch from which the training must resume.
    pub fn checkpoint(mut self, checkpoint: usize) -> Self {
        self.checkpoint = Some(checkpoint);
        self
    }

    /// Register a checkpointer that will save the [optimizer](crate::optim::Optimizer) and the
    /// [model](crate::module::Module) [states](crate::module::State).
    ///
    /// The number of checkpoints to be keep should be set to a minimum of two to be safe, since
    /// they are saved and deleted asynchronously and a crash during training might make a
    /// checkpoint non-usable.
    pub fn with_file_checkpointer<P: Element + serde::de::DeserializeOwned + serde::Serialize>(
        mut self,
        num_keep: usize,
    ) -> Self {
        self.checkpointer_model = Some(Arc::new(FileCheckpointer::<P>::new(
            format!("{}/checkpoint", self.directory).as_str(),
            "model",
            num_keep,
        )));
        self.checkpointer_optimizer = Some(Arc::new(FileCheckpointer::<P>::new(
            format!("{}/checkpoint", self.directory).as_str(),
            "optim",
            num_keep,
        )));
        self
    }

    /// Create the [learner](Learner) from a [module](ADModule) and an
    /// [optimizer](crate::optim::Optimizer).
    pub fn build<M, O>(self, model: M, optim: O) -> Learner<M, O, T, V>
    where
        M: ADModule<ADBackend = B>,
    {
        let callack = Box::new(self.dashboard);
        let callback = Box::new(AsyncTrainerCallback::new(callack));

        let create_checkpointer = |checkpointer| match checkpointer {
            Some(checkpointer) => {
                let checkpointer: Box<dyn Checkpointer<B::Elem>> =
                    Box::new(AsyncCheckpointer::new(checkpointer));
                Some(checkpointer)
            }
            None => None,
        };
        let mut model = model;
        model.detach();

        Learner {
            model,
            optim,
            num_epochs: self.num_epochs,
            callback,
            checkpoint: self.checkpoint,
            checkpointer_model: create_checkpointer(self.checkpointer_model),
            checkpointer_optimizer: create_checkpointer(self.checkpointer_optimizer),
        }
    }
}
