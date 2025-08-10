<script setup lang="ts">
import { onMounted, ref } from 'vue';
import * as interceptorWasm from "layer8-interceptor-production";
import { getCurrentInstance } from 'vue';
const { appContext } = getCurrentInstance();
const backend_url = appContext.config.globalProperties.$backend_url;

const images = ref<any[]>([]);
const searchName = ref<string>('');

function openImage(id: string) {
    interceptorWasm.fetch(`${backend_url}/images?id=${id}`)
        .then(response => response.json())
        .catch(err => {
            console.error('Error fetching image:', err);
        });
}

function searchImage() {
    if (!searchName.value) {
        fetchAllImages();
        return;
    }

    interceptorWasm.fetch(`${backend_url}/images?name=${searchName.value}`)
        .then(response => {
            if (!response.ok) throw new Error('Image not found');
            return response.json();
        })
        .then(data => {
            images.value = [{
                id: data.id,
                title: data.name,
                src: data.url
            }];
        })
        .catch(err => {
            alert(err.message);
            images.value = [];
        });
}

function fetchAllImages() {
    interceptorWasm.fetch(`${backend_url}/images`)
        .then(response => response.json())
        .then(data => {
            images.value = data.map((img: any) => ({
                id: img.id,
                title: img.name,
                src: img.url
            }));
        })
        .catch(err => {
            console.error('Error fetching images:', err);
        });
}

onMounted(() => {
    fetchAllImages();
});
</script>

<template>
    <div class="gallery-vertical">
        <h1>📸 Image Gallery</h1>

        <div class="search-container">
            <input v-model="searchName" type="text" placeholder="Enter Image Name" min="1" />
            <button @click="searchImage">Search</button>
            <button @click="fetchAllImages">Show All</button>
        </div>

        <div class="image-container">
            <div class="image-card" v-for="image in images" :key="image.id" @click="openImage(image.id)">
                <img :src="image.src" :alt="image.title" />
                <h2>{{ image.title }}</h2>
            </div>
        </div>
    </div>
</template>

<style scoped>
.search-container {
    margin: 20px 0;
    display: flex;
    gap: 10px;
    justify-content: center;
}

.search-container input {
    padding: 8px;
    border: 1px solid #ccc;
    border-radius: 4px;
    width: 150px;
}

.search-container button {
    padding: 8px 16px;
    background-color: #007bff;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
}

.search-container button:hover {
    background-color: #0056b3;
}

.image-card {
    /* Add these new styles */
    width: 300px;
    /* Fixed width */
    height: 300px;
    /* Fixed height - adjust as needed */
    margin: 10px;
    overflow: hidden;
    /* Hide any overflow from the image */
    cursor: pointer;
    border-radius: 8px;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
    transition: transform 0.3s ease;
}

.image-card:hover {
    transform: scale(1.03);
}

.image-card img {
    width: 100%;
    height: 80%;
    /* Adjust this percentage based on how much space you want for the image vs text */
    object-fit: cover;
    /* This will maintain aspect ratio while filling the space */
    display: block;
}

.image-card h2 {
    padding: 10px;
    font-size: 1rem;
    text-align: center;
    margin: 0;
}

/* Layout for the gallery */
.gallery-vertical {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 20px;
}

/* Add a container for the image cards */
.image-container {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 20px;
    width: 100%;
    max-width: 1200px;
}

.modal-content p {
    text-align: center;
    color: #666;
    margin-top: 0.5rem;
}

/* Keep all existing styles the same */
</style>
