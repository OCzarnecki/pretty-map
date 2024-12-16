from PIL import Image

DIR = "./output/planet_-0.418,51.37_0.268,51.647.osm.xz/"

grid_size = (3, 3)

filenames = [[f"{DIR}output_x{x}_y{y}.png" for y in range(grid_size[1])] for x in range(grid_size[0])]

images = [[Image.open(filenames[x][y]) for y in range(grid_size[1])] for x in range(grid_size[0])]

cell_width, cell_height = images[0][0].size

# Get the width and height of the images
total_width = sum(images[x][0].width for x in range(grid_size[0]))
total_height = sum(images[0][y].height for y in range(grid_size[1]))

# Create a new blank image with the appropriate size (total width and height)
combined_image = Image.new('RGBA', (total_width, total_height))

# Paste each image into its correct position
for x in range(grid_size[0]):
    for y in range(grid_size[1]):
        x_pos = x * cell_width
        y_pos = y * cell_height
        combined_image.paste(images[x][y], (x_pos, y_pos))

# Save the combined image
combined_image.save(f"{DIR}combined_output.tiff")
