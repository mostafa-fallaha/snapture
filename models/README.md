Place OCRS model files in this directory:

- `text-detection.rten`
- `text-recognition.rten`

If you want to use the default locations, fetch the model files into this folder:

```bash
mkdir -p models
curl -L https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten -o models/text-detection.rten
curl -L https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten -o models/text-recognition.rten
```