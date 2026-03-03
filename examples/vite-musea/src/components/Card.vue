<script setup lang="ts">
defineProps<{
  title: string;
  description?: string;
  image?: string;
  variant?: "default" | "outlined" | "elevated";
}>();
</script>

<template>
  <div class="card" :class="`card--${variant ?? "default"}`">
    <div v-if="image" class="card-image">
      <div class="card-image-placeholder" :style="{ background: image }">
        <span>
          {{ title.charAt(0) }}
        </span>
      </div>
    </div>
    <div class="card-body">
      <h3 class="card-title">
        {{ title }}
      </h3>
      <p v-if="description" class="card-description">
        {{ description }}
      </p>
      <div class="card-footer">
        <slot />
      </div>
    </div>
  </div>
</template>

<style scoped>
.card {
  border-radius: 8px;
  overflow: hidden;
  transition: transform .15s, box-shadow .15s;
  max-width: 320px;
  font-family: Helvetica Neue, Helvetica, Arial, sans-serif;
}

.card:hover {
  transform: translateY(-2px);
}

.card--default {
  background: #ddd9cd;
  border: 1px solid #c8c4b8;
}

.card--outlined {
  background: none;
  border: 2px solid #c8c4b8;
}

.card--elevated {
  background: #ddd9cd;
  border: none;
  box-shadow: 0 4px 16px #1212120f;
}

.card--elevated:hover {
  box-shadow: 0 8px 24px #1212121a;
}

.card-image {
  aspect-ratio: 16 / 9;
  overflow: hidden;
}

.card-image-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 2rem;
  font-weight: 700;
  color: #e6e2d6cc;
}

.card-body {
  padding: 1rem 1.25rem;
}

.card-title {
  font-size: 1rem;
  font-weight: 600;
  color: #121212;
  margin: 0 0 .5rem;
}

.card-description {
  font-size: .875rem;
  color: #6b6b6b;
  margin: 0 0 1rem;
  line-height: 1.5;
}

.card-footer {
  display: flex;
  gap: .5rem;
}
</style>

<art>
<variant name="Default" default>
    <Self title="Getting Started" description="Learn how to build with Musea components.">
      <button class="btn btn--primary">Read More</button>
    </Self>
  </variant>
  <variant name="With Image">
    <Self
      title="Featured"
      description="A card with an image header for rich content display."
      image="#6b5090"
    >
      <button class="btn btn--primary">View</button>
      <button class="btn">Share</button>
    </Self>
  </variant>
  <variant name="Outlined">
    <Self title="Outlined Card" description="A card with an outlined style." variant="outlined">
      <button class="btn">Action</button>
    </Self>
  </variant>
  <variant name="Elevated">
    <Self title="Elevated Card" description="A card with elevation shadow." variant="elevated">
      <button class="btn btn--primary">Action</button>
    </Self>
  </variant>
</art>
