from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from PIL import Image
from transformers import pipeline

app = FastAPI()

classifier = pipeline("image-classification", model="Falconsai/nsfw_image_detection", use_fast=True)

class ScanRequest(BaseModel):
    path: str

@app.get("/health")
def read_root():
    return {"ok": True}

@app.post("/scan")
async def scan_image(request: ScanRequest):
    try:
        img = Image.open(request.path)
        result = classifier(img)
        nsfw_score = next((r["score"] for r in result if r["label"] == "nsfw"), 0.0)
        return {"score": nsfw_score}
    except FileNotFoundError:
        raise HTTPException(status_code=404, detail="File not found")
    except Exception as e:
        raise HTTPException(status_code=400, detail=str(e))

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=4100)
