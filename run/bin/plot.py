import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import AutoMinorLocator

df0 = pd.read_csv("../benchresults/benches-pack.csv")

df = df0[ (df0['time_ratio'] < 0.5) ]

compression_time = np.array(df['ns']/(1e+6))

compression_ratio = np.array(df['compression_ratio'])

compression_name = np.array(df['bench'])

for i in range(len(compression_name)):
    compression_name[i] = compression_name[i].replace('compression/', '').replace('.pack', '')

# Set ggplot styles and update Matplotlib with them.
ggplot_styles = {
    'axes.edgecolor': 'white',
    'axes.facecolor': 'EBEBEB',
    'axes.grid': True,
    'axes.grid.which': 'both',
    'axes.spines.left': False,
    'axes.spines.right': False,
    'axes.spines.top': False,
    'axes.spines.bottom': False,
    'grid.color': 'white',
    'grid.linewidth': '1.2',
    'xtick.color': '555555',
    'xtick.major.bottom': True,
    'xtick.minor.bottom': False,
    'ytick.color': '555555',
    'ytick.major.left': True,
    'ytick.minor.left': False,
}

plt.rcParams.update(ggplot_styles)

fig, ax = plt.subplots()

# Set minor ticks/gridline cadence.
ax.xaxis.set_minor_locator(AutoMinorLocator(2))
ax.yaxis.set_minor_locator(AutoMinorLocator(2))

# Turn minor gridlines on and make them thinner.
ax.grid(which='minor', linewidth=0.6)

# plt.yscale('log')
plt.ylabel("Time (ms)")
plt.xlabel("Compression ratio (original size / compressed size)")

# ax.scatter(compression_ratio, compression_time)

# # # zip joins x and y coordinates in pairs
# for x,y,label in zip(compression_ratio, compression_time, compression_name):
#     plt.annotate(label.split('/')[1], # this is the text
#                  (x,y), # these are the coordinates to position the label
#                  textcoords="offset points", # how to position the text
#                  xytext=(0,10), # distance from text to points (x,y)
#                  ha='center') # horizontal alignment can be left, right or center

# # zip joins x and y coordinates in pairs
for x,y,label in zip(compression_ratio, compression_time, compression_name):
    ax.plot(x, y, 'o', label=label.split('/')[1])

plt.legend()
# plt.legend(bbox_to_anchor=(0,-0.5,1,0.5), loc='upper left')
ax.legend(loc='lower left', bbox_to_anchor= (0.0, 1.01, 1, 1), ncol=3,
            borderaxespad=0, frameon=False)

plt.savefig('pack.png', bbox_inches='tight')
