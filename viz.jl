
using CSV, Gadfly, DataFrames

df = CSV.read("grow-bets.csv", DataFrame)
gdf = groupby(df, :label)
df = transform(gdf, eachindex => :i)
plot(transform(df, :rolls => ByRow(x -> log(1.0 * x)) => :log_rolls),
     x=:i,
     y=:log_rolls,
     color=:label,
     Geom.line,
     Guide.title("sorted ln rolls"))
plot(transform(df, :max_bankroll => ByRow(x -> log(1.0 * x)) => :log_maxbank),
     x=:i,
     y=:log_maxbank,
     color=:label,
     Geom.line,
     Guide.title("sorted ln max bankrolls"))

plot(filter(:i => (x -> x <= 650), df),
     x=:i, y=:rolls, color=:label, Geom.line,
     Guide.title("lowest 650 roll counts"))
plot(filter(:i => (x -> x <= 650), df),
     x=:i, y=:max_bankroll, color=:label, Geom.line,
     Guide.title("lowest 650 roll max bankrolls"))
