import React, { useState, useEffect } from 'react';
import { Text, View, StyleSheet, Button, ActivityIndicator, ScrollView } from 'react-native';
import { CameraView, Camera } from 'expo-camera';
import { api } from '../api/spoils';

export default function ScanScreen() {
  const [hasPermission, setHasPermission] = useState<boolean | null>(null);
  const [scanned, setScanned] = useState(false);
  const [loading, setLoading] = useState(false);
  const [product, setProduct] = useState<any>(null);
  const [error, setError] = useState<string>('');

  useEffect(() => {
    const getCameraPermissions = async () => {
      const { status } = await Camera.requestCameraPermissionsAsync();
      setHasPermission(status === 'granted');
    };

    getCameraPermissions();
  }, []);

  const handleBarCodeScanned = async ({ type, data }: { type: string; data: string }) => {
    setScanned(true);
    setLoading(true);
    setError('');
    setProduct(null);

    try {
      const result = await api.getProduct(data);
      setProduct(result);
    } catch (err) {
      setError(`Failed to fetch product: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  if (hasPermission === null) {
    return <View style={styles.container}><Text>Requesting camera permission...</Text></View>;
  }
  if (hasPermission === false) {
    return <View style={styles.container}><Text>No access to camera</Text></View>;
  }

  return (
    <View style={styles.container}>
      {!scanned && (
        <CameraView
          style={styles.camera}
          onBarcodeScanned={scanned ? undefined : handleBarCodeScanned}
          barcodeScannerSettings={{
            barcodeTypes: ['ean13', 'ean8', 'upc_a', 'upc_e'],
          }}
        />
      )}

      {loading && (
        <View style={styles.loadingContainer}>
          <ActivityIndicator size="large" color="#0000ff" />
          <Text style={styles.loadingText}>Loading product...</Text>
        </View>
      )}

      {product && (
        <ScrollView style={styles.productContainer}>
          <Text style={styles.productName}>{product.product_name || 'Unknown Product'}</Text>
          <Text style={styles.productBrand}>{product.brands || 'Unknown Brand'}</Text>

          {product.nutriscore_grade && (
            <View style={styles.badge}>
              <Text style={styles.badgeText}>Nutriscore: {product.nutriscore_grade.toUpperCase()}</Text>
            </View>
          )}

          {product.nova_group && (
            <View style={styles.badge}>
              <Text style={styles.badgeText}>Nova Group: {product.nova_group}</Text>
            </View>
          )}

          {product.categories && (
            <View style={styles.section}>
              <Text style={styles.sectionTitle}>Categories:</Text>
              <Text style={styles.sectionText}>{product.categories}</Text>
            </View>
          )}

          {product.ingredients_text && (
            <View style={styles.section}>
              <Text style={styles.sectionTitle}>Ingredients:</Text>
              <Text style={styles.sectionText}>{product.ingredients_text}</Text>
            </View>
          )}

          {product.allergens && (
            <View style={styles.section}>
              <Text style={styles.sectionTitle}>Allergens:</Text>
              <Text style={styles.alertText}>{product.allergens}</Text>
            </View>
          )}
        </ScrollView>
      )}

      {error && <Text style={styles.errorText}>{error}</Text>}

      {scanned && (
        <Button title="Scan Again" onPress={() => {
          setScanned(false);
          setProduct(null);
          setError('');
        }} />
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    flexDirection: 'column',
    justifyContent: 'center',
  },
  camera: {
    flex: 1,
  },
  loadingContainer: {
    padding: 20,
    alignItems: 'center',
  },
  loadingText: {
    marginTop: 10,
    fontSize: 16,
  },
  productContainer: {
    flex: 1,
    padding: 20,
    backgroundColor: '#fff',
  },
  productName: {
    fontSize: 24,
    fontWeight: 'bold',
    marginBottom: 8,
  },
  productBrand: {
    fontSize: 18,
    color: '#666',
    marginBottom: 16,
  },
  badge: {
    backgroundColor: '#007AFF',
    padding: 8,
    borderRadius: 8,
    marginBottom: 8,
    alignSelf: 'flex-start',
  },
  badgeText: {
    color: '#fff',
    fontWeight: 'bold',
  },
  section: {
    marginTop: 16,
  },
  sectionTitle: {
    fontSize: 16,
    fontWeight: 'bold',
    marginBottom: 4,
  },
  sectionText: {
    fontSize: 14,
    color: '#333',
  },
  alertText: {
    fontSize: 14,
    color: '#ff0000',
    fontWeight: 'bold',
  },
  errorText: {
    color: '#dc3545',
    padding: 20,
    textAlign: 'center',
  },
});
