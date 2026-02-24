from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import Dict, Optional, List

app = FastAPI(title="Simple Product REST Service")


class Product(BaseModel):
    id: int
    name: str
    description: str


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
        description=product.description
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
