from fastapi import FastAPI, UploadFile, File, HTTPException
from fastapi.responses import FileResponse
from pydantic import BaseModel
from typing import Dict, Optional, List
from pathlib import Path
import shutil

app = FastAPI(title="Funny title")


class Product(BaseModel):
    id: int
    name: str
    description: str
    icon: Optional[str]


class ProductProject(BaseModel):
    name: Optional[str] = None
    description: Optional[str] = None


products: Dict[int, Product] = {}
next_id: int = 1


@app.post("/product", response_model=Product)
def create_product(product: ProductProject):
    global next_id
    print(product)
    new_product = Product(
        id=next_id,
        name=product.name,
        description=product.description,
        icon=None
    )
    products[next_id] = new_product
    next_id += 1

    return new_product


@app.get("/product/{product_id}", response_model=Product)
def get_product(product_id: int):
    product = products.get(product_id)
    if not product:
        raise HTTPException(status_code=404, detail="Product not found")
    return product


@app.put("/product/{product_id}", response_model=Product)
def update_product(product_id: int, update: ProductProject):
    product = products.get(product_id)
    if not product:
        raise HTTPException(status_code=404, detail="Product not found")

    updated_data = product.dict()

    if update.name is not None:
        updated_data["name"] = update.name
    if update.description is not None:
        updated_data["description"] = update.description

    updated_product = Product(**updated_data)
    products[product_id] = updated_product

    return updated_product


@app.delete("/product/{product_id}", response_model=Product)
def delete_product(product_id: int):
    product = products.pop(product_id, None)
    if not product:
        raise HTTPException(status_code=404, detail="Product not found")
    return product


@app.get("/products", response_model=List[Product])
def get_all_products():
    return list(products.values())


IMAGES_DIR = Path("images/products")


@app.post("/product/{product_id}/image")
def upload_product_image(
        product_id: int,
        image: UploadFile = File(...)
):
    product = products.get(product_id)
    if not product:
        raise HTTPException(status_code=404, detail="Product not found")

    product_dir = IMAGES_DIR / str(product_id)
    product_dir.mkdir(parents=True, exist_ok=True)

    file_path = product_dir / image.filename

    with file_path.open("wb") as file:
        shutil.copyfileobj(image.file, file)

    product.icon = str(file_path)
    products[product_id] = product

    return {
        "message": "Image uploaded successfully",
        "icon": product.icon
    }


@app.get("/product/{product_id}/image")
def get_product_image(product_id: int):
    product = products.get(product_id)
    if not product or not product.icon:
        raise HTTPException(status_code=404, detail="Image not found")

    return FileResponse(
        path=product.icon,
        media_type="image/png"
    )
