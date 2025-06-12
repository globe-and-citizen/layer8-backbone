<script setup lang="ts">
import {onMounted, ref} from 'vue';
import {wasmBackend, getToken, toImageUrl, toBlob, revokeURL} from '@/utils.js';
import {save_image, get_image} from "interceptor-wasm"

const images = ref<any[]>([]);
const selectedImage = ref<any | null>(null); // For modal
const showModal = ref(false);

onMounted(() => {
    let token = getToken('jwt') || "";

    wasmBackend.get_images(null, token)
        .then(data => {
            let list = data.images || data["images"] || data.get("images");

            for (let i = 0; i < list.length; i++) {
                let bytes = list[i].content || list[i]["content"] || list[i].get("content");
                let filename = list[i].file_name || list[i]["file_name"] || list[i].get("file_name");

                let image = {
                    id: list[i].id || list[i]["id"] || list[i].get("id"),
                    title: list[i].title || list[i]["title"] || list[i].get("title"),
                    filename: filename,
                    src: toImageUrl(filename, bytes),
                }
                images.value.push(image);
            }
        }).catch(err => {
        console.error('Error fetching profile:', err);
    })
});

function openImage(id: string, title: string = "") {
    const token = getToken('jwt') || "";

    get_image(title).then(data => {
        if (data) {
            selectedImage.value = {
                id: id,
                title: title,
                src: URL.createObjectURL(data),
            };
        } else {
            wasmBackend.get_images(id, token)
                .then(data => {
                    let bytes = data.content || data["content"] || data.get("content");
                    let filename = data.file_name || data["file_name"] || data.get("file_name");
                    console.log("image", data)
                    selectedImage.value = {
                        id: data.id || data["id"] || data.get("id"),
                        title: data.title || data["title"] || data.get("title"),
                        filename: filename,
                        src: toImageUrl(filename, bytes),
                    };

                    save_image(selectedImage.value.title, toBlob(filename, bytes) as Blob)
                }).catch(err => {
                console.error('Error fetching full image:', err);
            })
        }
        showModal.value = true;
    }).catch(err => {
        console.error('Error fetching full image:', err);
    })
}

function closeModal() {
    showModal.value = false;
    selectedImage.value = null;
}
</script>

<template>
    <div class="gallery-vertical">
        <h1>📸 Image Gallery</h1>
        <div
            class="image-card"
            v-for="image in images"
            :key="image.id"
            @click="openImage(`${image.id}`, image.title)"
        >
            <img :src="image.src" alt="Click to load image" @load="revokeURL(image.src)"/>
            <h2>{{ image.title }}</h2>
        </div>
    </div>

    <!-- Modal -->
    <div v-if="showModal" class="modal-overlay" @click.self="closeModal">
        <div class="modal-content">
            <button class="close-button" @click="closeModal">✖</button>
            <img :src="selectedImage?.src" :alt="selectedImage?.title" @load="revokeURL(selectedImage?.src)"/>
            <h2>{{ selectedImage?.title }}</h2>
        </div>
    </div>
</template>

<style scoped>
.gallery-vertical {
    max-width: 600px;
    margin: 2rem auto;
    display: flex;
    flex-direction: column;
    align-items: center;
}

.image-card {
    margin-bottom: 2rem;
    background: #fff;
    border-radius: 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    overflow: hidden;
    width: 100%;
    cursor: pointer;
    transition: transform 0.2s;
}

.image-card:hover {
    transform: scale(1.02);
}

.image-card img {
    width: 100%;
    height: auto;
    object-fit: cover;
}

.image-card h2 {
    padding: 1rem;
    font-size: 1.1rem;
    text-align: center;
}

/* Modal Styles */
.modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.6);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 999;
}

.modal-content {
    background: #fff;
    border-radius: 12px;
    padding: 1rem;
    max-width: 90%;
    max-height: 90%;
    overflow: auto;
    text-align: center;
    position: relative;
}

.modal-content img {
    max-width: 100%;
    height: auto;
    margin-bottom: 1rem;
}

.modal-content h2 {
    font-size: 1.2rem;
}

.close-button {
    position: absolute;
    top: 0.5rem;
    right: 1rem;
    background: transparent;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
}
</style>
