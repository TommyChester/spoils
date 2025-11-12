const API_URL = process.env.EXPO_PUBLIC_API_URL || 'http://localhost:8080';

export interface HealthResponse {
  status: string;
  message: string;
}

export interface Product {
  id: number;
  barcode: string;
  product_name?: string;
  brands?: string;
  categories?: string;
  quantity?: string;
  image_url?: string;
  nutriscore_grade?: string;
  nova_group?: number;
  ecoscore_grade?: string;
  ingredients_text?: string;
  allergens?: string;
  full_response: any;
  created_at: string;
  updated_at: string;
}

export const api = {
  async health(): Promise<HealthResponse> {
    const response = await fetch(`${API_URL}/health`);
    if (!response.ok) {
      throw new Error('Health check failed');
    }
    return response.json();
  },

  async hello(): Promise<{ message: string }> {
    const response = await fetch(`${API_URL}/api/hello`);
    if (!response.ok) {
      throw new Error('API call failed');
    }
    return response.json();
  },

  async getProduct(barcode: string): Promise<Product> {
    const response = await fetch(`${API_URL}/api/products/${barcode}`);
    if (!response.ok) {
      throw new Error(`Product lookup failed: ${response.statusText}`);
    }
    return response.json();
  },
};
