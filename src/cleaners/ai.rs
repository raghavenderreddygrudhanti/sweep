//! AI/ML cache cleaner — the feature Mole doesn't have.
//! Cleans: HuggingFace, Ollama, torch, pip, conda, model downloads.

use std::path::PathBuf;

/// AI/ML cache locations that can grow to 20-100+ GB.
pub fn ai_cache_paths() -> Vec<(PathBuf, &'static str)> {
    let home = crate::error::home_or_exit();

    vec![
        (
            home.join(".cache/huggingface"),
            "HuggingFace models & datasets",
        ),
        (home.join(".cache/torch"), "PyTorch model cache"),
        (home.join(".cache/pip"), "pip package cache"),
        (home.join(".ollama/models"), "Ollama downloaded models"),
        (home.join(".cache/conda"), "Conda package cache"),
        (home.join(".conda/pkgs"), "Conda packages"),
        (home.join("miniconda3/pkgs"), "Miniconda packages"),
        (home.join("anaconda3/pkgs"), "Anaconda packages"),
        (home.join(".cache/whisper"), "Whisper model cache"),
        (home.join(".cache/clip"), "CLIP model cache"),
        (home.join(".triton/cache"), "Triton compilation cache"),
        (home.join(".cache/matplotlib"), "Matplotlib cache"),
        (home.join(".keras/models"), "Keras model cache"),
        (home.join("Library/Caches/com.lmstudio"), "LM Studio cache"),
        (home.join(".cache/lm-studio"), "LM Studio models (Linux)"),
    ]
    .into_iter()
    .filter(|(p, _)| p.exists())
    .collect()
}
