<script setup lang="ts">
import {onMounted, ref} from 'vue';

const images = ref<any[]>([]);
const selectedImage = ref<any | null>(null);
const showModal = ref(false);
const searchId = ref<string>('');

function openImage(id: string) {
    fetch(`http://localhost:6191/images?id=${id}`)
        .then(response => response.json())
        .then(data => {
            selectedImage.value = {
                id: data.id,
                title: data.name,
                src: data.url
            };
            showModal.value = true;
        })
        .catch(err => {
            console.error('Error fetching image:', err);
        });
}

function closeModal() {
    showModal.value = false;
    selectedImage.value = null;
}

function searchImage() {
    if (!searchId.value) {
        fetchAllImages();
        return;
    }
    
    fetch(`http://localhost:6191/images?id=${searchId.value}`)
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
    fetch('http://localhost:6191/images')
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
            <input 
                v-model="searchId" 
                type="number" 
                placeholder="Enter Image ID"
                min="1"
            />
            <button @click="searchImage">Search</button>
            <button @click="fetchAllImages">Show All</button>
        </div>
        
        <div
            class="image-card"
            v-for="image in images"
            :key="image.id"
            @click="openImage(image.id)"
        >
            <img :src="image.src" :alt="image.title"/>
            <h2>{{ image.title }}</h2>
            <p>ID: {{ image.id }}</p>
        </div>

        <!-- Modal -->
        <div v-if="showModal" class="modal-overlay" @click.self="closeModal">
            <div class="modal-content">
                <button class="close-button" @click="closeModal">✖</button>
                <img :src="selectedImage?.src" :alt="selectedImage?.title"/>
                <h2>{{ selectedImage?.title }}</h2>
                <p>ID: {{ selectedImage?.id }}</p>
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

.image-card p {
    text-align: center;
    color: #666;
    margin-top: 0.5rem;
}

.modal-content p {
    text-align: center;
    color: #666;
    margin-top: 0.5rem;
}

/* Keep all existing styles the same */
</style>